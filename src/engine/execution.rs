use crate::engine::entry_policies::resolve_entry_policy;
use crate::engine::types::{
    risk_amount, stop_hit, target_hit, EntryModel, EntryPolicy, ExecutionConfig, OpenPosition,
    SetupCandidate, StopModel, TargetModel, TrailingModel,
};
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutionMetrics {
    pub setup_count: usize,
    pub limit_orders_placed: usize,
    pub limit_orders_filled: usize,
    pub limit_orders_expired: usize,
    pub skipped_open_same_dir: usize,
    pub skipped_open_opposite_dir: usize,
    pub skipped_other: usize,
}

pub fn run_setups(
    candles: &[CandleStick],
    setups: &[SetupCandidate],
    cfg: &ExecutionConfig,
) -> Vec<Trade> {
    run_setups_with_metrics(candles, setups, cfg).0
}

pub fn run_setups_with_metrics(
    candles: &[CandleStick],
    setups: &[SetupCandidate],
    cfg: &ExecutionConfig,
) -> (Vec<Trade>, ExecutionMetrics) {
    let mut trades = Vec::new();
    let mut metrics = ExecutionMetrics {
        setup_count: setups.len(),
        ..ExecutionMetrics::default()
    };
    let mut open: Option<OpenPosition> = None;
    let mut setup_idx = 0usize;
    let mut pending_market: Option<(SetupCandidate, usize)> = None;
    let mut pending_limit: Option<(SetupCandidate, usize, EntryPolicy)> = None;

    for (i, c) in candles.iter().copied().enumerate() {
        let mut opened_this_bar = false;

        if open.is_none() {
            if let Some((candidate, entry_index)) = pending_market {
                if i >= entry_index {
                    open = build_position(candidate, i, c.open, c.open_time);
                    if open.is_some() {
                        opened_this_bar = true;
                    }
                    pending_market = None;
                }
            }
        }

        while setup_idx < setups.len() && setups[setup_idx].signal_index < i {
            metrics.skipped_other += 1;
            setup_idx += 1;
        }

        if open.is_none() && pending_market.is_none() {
            if let Some((candidate, expires_at, policy)) = pending_limit {
                if i > expires_at {
                    metrics.limit_orders_expired += 1;
                    pending_limit = None;
                } else {
                    let previous = if i > 0 { candles[i - 1] } else { c };
                    let price = resolve_entry_policy(policy, candidate.direction, c, previous);
                    let invalidated = candidate_stop_hit(c, candidate);
                    let touched = c.low <= price && c.high >= price;
                    let traded_through = match candidate.direction {
                        crate::model::position_direction::PositionDirection::Long => {
                            c.low <= DecimalVec(price.0 - cfg.tick_size)
                        }
                        crate::model::position_direction::PositionDirection::Short => {
                            c.high >= DecimalVec(price.0 + cfg.tick_size)
                        }
                    };
                    let fillable = match candidate.entry {
                        EntryModel::LimitTradeThrough { .. } => traded_through,
                        _ => touched,
                    };
                    if invalidated && !touched {
                        metrics.limit_orders_expired += 1;
                        pending_limit = None;
                    } else if fillable {
                        if let Some(p) =
                            build_position(candidate, i, DecimalVec(price.0), c.open_time)
                        {
                            metrics.limit_orders_filled += 1;
                            if invalidated || stop_hit(c, &p) {
                                let stop_fill = gap_aware_stop_fill(c, &p);
                                trades.push(build_trade(
                                    p,
                                    c.close_time,
                                    stop_fill,
                                    stop_result(&p),
                                    cfg,
                                ));
                            } else {
                                open = Some(p);
                                opened_this_bar = true;
                            }
                        }
                        pending_limit = None;
                    }
                }
            }

            while open.is_none() && setup_idx < setups.len() && setups[setup_idx].signal_index == i
            {
                let candidate = setups[setup_idx];
                match candidate.entry {
                    EntryModel::SignalClose | EntryModel::MarketClose => {
                        open = build_position(candidate, i, c.close, c.close_time);
                        if open.is_some() {
                            opened_this_bar = true;
                        }
                    }
                    EntryModel::NextBarOpen => {
                        if i + 1 < candles.len() {
                            pending_market = Some((candidate, i + 1));
                        }
                    }
                    EntryModel::LimitTouch { expiry_bars, .. } => {
                        if let EntryModel::LimitTouch { price, .. } = candidate.entry {
                            metrics.limit_orders_placed += 1;
                            pending_limit =
                                Some((candidate, i + expiry_bars, EntryPolicy::Price(price)));
                        }
                    }
                    EntryModel::LimitTradeThrough { expiry_bars, .. } => {
                        if let EntryModel::LimitTradeThrough { price, .. } = candidate.entry {
                            metrics.limit_orders_placed += 1;
                            pending_limit =
                                Some((candidate, i + expiry_bars, EntryPolicy::Price(price)));
                        }
                    }
                    EntryModel::LimitByPolicy {
                        policy,
                        expiry_bars,
                    } => {
                        metrics.limit_orders_placed += 1;
                        pending_limit = Some((candidate, i + expiry_bars, policy));
                    }
                }
                setup_idx += 1;
            }
        } else {
            while setup_idx < setups.len() && setups[setup_idx].signal_index == i {
                let candidate = setups[setup_idx];
                if let Some(p) = open {
                    if candidate.direction == p.direction {
                        metrics.skipped_open_same_dir += 1;
                    } else {
                        metrics.skipped_open_opposite_dir += 1;
                    }
                } else {
                    metrics.skipped_other += 1;
                }
                setup_idx += 1;
            }
        }

        if let Some(mut p) = open {
            if opened_this_bar {
                open = Some(p);
                continue;
            }

            apply_trailing(&mut p, c);
            if stop_hit(c, &p) {
                let stop_fill = gap_aware_stop_fill(c, &p);
                trades.push(build_trade(
                    p,
                    c.close_time,
                    stop_fill,
                    stop_result(&p),
                    cfg,
                ));
                open = None;
            } else if target_hit(c, &p) {
                let tp_fill = p.target;
                trades.push(build_trade(
                    p,
                    c.close_time,
                    tp_fill,
                    TradeResult::Winner,
                    cfg,
                ));
                open = None;
            } else {
                if let Some(max_hold_bars) = p.max_hold_bars {
                    if i.saturating_sub(p.entry_index) >= max_hold_bars {
                        let exit_px = c.close;
                        let result = time_exit_result(&p, exit_px);
                        trades.push(build_trade(p, c.close_time, exit_px, result, cfg));
                        open = None;
                        continue;
                    }
                }
                open = Some(p);
            }
        }
    }

    if pending_limit.is_some() {
        metrics.limit_orders_expired += 1;
    }
    if pending_market.is_some() {
        metrics.skipped_other += 1;
    }

    (trades, metrics)
}

fn build_position(
    candidate: SetupCandidate,
    entry_index: usize,
    raw_entry: DecimalVec,
    open_time: i64,
) -> Option<OpenPosition> {
    let entry = raw_entry;
    let stop = match candidate.stop {
        StopModel::FixedPrice(px) => px,
    };
    let risk = risk_amount(candidate.direction, entry, stop);
    if risk <= Decimal::ZERO {
        return None;
    }
    let target = match candidate.target {
        TargetModel::FixedPrice(px) => px,
        TargetModel::FixedR(r) => match candidate.direction {
            crate::model::position_direction::PositionDirection::Long => {
                DecimalVec(entry.0 + risk * r)
            }
            crate::model::position_direction::PositionDirection::Short => {
                DecimalVec(entry.0 - risk * r)
            }
        },
        TargetModel::FixedPoints(points) => match candidate.direction {
            crate::model::position_direction::PositionDirection::Long => {
                DecimalVec(entry.0 + points)
            }
            crate::model::position_direction::PositionDirection::Short => {
                DecimalVec(entry.0 - points)
            }
        },
    };
    Some(OpenPosition {
        direction: candidate.direction,
        entry_index,
        open_time,
        entry,
        stop,
        current_stop: stop,
        target,
        initial_risk: risk,
        trailing: candidate.trailing,
        max_hold_bars: candidate.max_hold_bars,
    })
}

fn candidate_stop_hit(c: CandleStick, candidate: SetupCandidate) -> bool {
    let stop = match candidate.stop {
        StopModel::FixedPrice(px) => px,
    };
    match candidate.direction {
        crate::model::position_direction::PositionDirection::Long => c.low <= stop,
        crate::model::position_direction::PositionDirection::Short => c.high >= stop,
    }
}

fn stop_result(p: &OpenPosition) -> TradeResult {
    if p.current_stop == p.entry {
        TradeResult::BreakEven
    } else if (p.direction == crate::model::position_direction::PositionDirection::Long
        && p.current_stop > p.entry)
        || (p.direction == crate::model::position_direction::PositionDirection::Short
            && p.current_stop < p.entry)
    {
        TradeResult::Winner
    } else {
        TradeResult::Expense
    }
}

fn time_exit_result(p: &OpenPosition, exit_px: DecimalVec) -> TradeResult {
    match p.direction {
        crate::model::position_direction::PositionDirection::Long => {
            if exit_px.0 > p.entry.0 {
                TradeResult::Winner
            } else if exit_px.0 < p.entry.0 {
                TradeResult::Expense
            } else {
                TradeResult::BreakEven
            }
        }
        crate::model::position_direction::PositionDirection::Short => {
            if exit_px.0 < p.entry.0 {
                TradeResult::Winner
            } else if exit_px.0 > p.entry.0 {
                TradeResult::Expense
            } else {
                TradeResult::BreakEven
            }
        }
    }
}

fn apply_trailing(position: &mut OpenPosition, c: CandleStick) {
    match position.trailing {
        TrailingModel::None => {}
        TrailingModel::BreakEvenAtR(threshold_r) => {
            let attained_r = match position.direction {
                crate::model::position_direction::PositionDirection::Long => {
                    (c.high.0 - position.entry.0) / position.initial_risk
                }
                crate::model::position_direction::PositionDirection::Short => {
                    (position.entry.0 - c.low.0) / position.initial_risk
                }
            };
            if attained_r >= threshold_r {
                position.current_stop = position.entry;
            }
        }
        TrailingModel::StepAtR { trigger_r, lock_r } => {
            let attained_r = match position.direction {
                crate::model::position_direction::PositionDirection::Long => {
                    (c.high.0 - position.entry.0) / position.initial_risk
                }
                crate::model::position_direction::PositionDirection::Short => {
                    (position.entry.0 - c.low.0) / position.initial_risk
                }
            };
            if attained_r >= trigger_r {
                let new_sl = match position.direction {
                    crate::model::position_direction::PositionDirection::Long => {
                        DecimalVec(position.entry.0 + lock_r * position.initial_risk)
                    }
                    crate::model::position_direction::PositionDirection::Short => {
                        DecimalVec(position.entry.0 - lock_r * position.initial_risk)
                    }
                };
                match position.direction {
                    crate::model::position_direction::PositionDirection::Long => {
                        if new_sl > position.current_stop {
                            position.current_stop = new_sl;
                        }
                    }
                    crate::model::position_direction::PositionDirection::Short => {
                        if new_sl < position.current_stop {
                            position.current_stop = new_sl;
                        }
                    }
                }
            }
        }
        TrailingModel::ProgressiveHalfR { start_r, step_r } => {
            let attained_r = match position.direction {
                crate::model::position_direction::PositionDirection::Long => {
                    (c.high.0 - position.entry.0) / position.initial_risk
                }
                crate::model::position_direction::PositionDirection::Short => {
                    (position.entry.0 - c.low.0) / position.initial_risk
                }
            };
            if attained_r >= start_r {
                let lock_r = if attained_r >= start_r + step_r {
                    let excess = attained_r - start_r;
                    let steps = (excess / step_r).floor();
                    steps * step_r
                } else {
                    Decimal::ZERO
                };
                let new_sl = match position.direction {
                    crate::model::position_direction::PositionDirection::Long => {
                        DecimalVec(position.entry.0 + lock_r * position.initial_risk)
                    }
                    crate::model::position_direction::PositionDirection::Short => {
                        DecimalVec(position.entry.0 - lock_r * position.initial_risk)
                    }
                };
                match position.direction {
                    crate::model::position_direction::PositionDirection::Long => {
                        if new_sl > position.current_stop {
                            position.current_stop = new_sl;
                        }
                    }
                    crate::model::position_direction::PositionDirection::Short => {
                        if new_sl < position.current_stop {
                            position.current_stop = new_sl;
                        }
                    }
                }
            }
        }
        TrailingModel::PreviousClosePoints {
            trigger_points,
            distance_points,
        } => {
            let reference = c.close.0;
            let profit_points = match position.direction {
                crate::model::position_direction::PositionDirection::Long => {
                    reference - position.entry.0
                }
                crate::model::position_direction::PositionDirection::Short => {
                    position.entry.0 - reference
                }
            };
            if profit_points < trigger_points {
                return;
            }
            match position.direction {
                crate::model::position_direction::PositionDirection::Long => {
                    let candidate = DecimalVec(reference - distance_points);
                    if candidate > position.current_stop {
                        position.current_stop = candidate;
                    }
                }
                crate::model::position_direction::PositionDirection::Short => {
                    let candidate = DecimalVec(reference + distance_points);
                    if candidate < position.current_stop {
                        position.current_stop = candidate;
                    }
                }
            }
        }
    }
}

fn gap_aware_stop_fill(c: CandleStick, p: &OpenPosition) -> DecimalVec {
    match p.direction {
        crate::model::position_direction::PositionDirection::Long => {
            if c.open <= p.current_stop {
                DecimalVec(c.open.0)
            } else {
                p.current_stop
            }
        }
        crate::model::position_direction::PositionDirection::Short => {
            if c.open >= p.current_stop {
                DecimalVec(c.open.0)
            } else {
                p.current_stop
            }
        }
    }
}

fn build_trade(
    p: OpenPosition,
    close_time: i64,
    exit_px: DecimalVec,
    result: TradeResult,
    cfg: &ExecutionConfig,
) -> Trade {
    let notional = (p.entry.0 + exit_px.0) / Decimal::from(2);
    let commission = notional * cfg.commission_rate_per_side * Decimal::from(2);
    let fees = notional * cfg.fee_rate_per_side * Decimal::from(2);
    let slippage = (cfg.tick_size
        * Decimal::from_i32(cfg.slippage_ticks_per_side).unwrap_or(Decimal::ZERO))
    .abs()
        * Decimal::from(2);
    Trade {
        direction: p.direction,
        open_time: p.open_time,
        close_time,
        entry: p.entry,
        sl: p.stop,
        tp: exit_px,
        result,
        commission,
        slippage,
        fees,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::position_direction::PositionDirection;

    fn dv(value: i64) -> DecimalVec {
        DecimalVec(Decimal::from(value))
    }

    fn candle(index: i64, open: i64, high: i64, low: i64, close: i64) -> CandleStick {
        CandleStick {
            open_time: index * 60,
            close_time: index * 60 + 59,
            open: dv(open),
            high: dv(high),
            low: dv(low),
            close: dv(close),
        }
    }

    fn long_setup(entry: EntryModel) -> SetupCandidate {
        SetupCandidate {
            direction: PositionDirection::Long,
            signal_index: 0,
            entry,
            stop: StopModel::FixedPrice(dv(95)),
            target: TargetModel::FixedPrice(dv(110)),
            trailing: TrailingModel::None,
            max_hold_bars: None,
        }
    }

    #[test]
    fn next_bar_open_does_not_exit_on_entry_bar() {
        let candles = vec![
            candle(0, 100, 101, 99, 100),
            candle(1, 100, 101, 90, 96),
            candle(2, 96, 110, 96, 110),
        ];
        let setup = long_setup(EntryModel::NextBarOpen);

        let trades = run_setups(&candles, &[setup], &ExecutionConfig::default());

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].result, TradeResult::Winner);
        assert_eq!(trades[0].open_time, candles[1].open_time);
        assert_eq!(trades[0].close_time, candles[2].close_time);
    }

    #[test]
    fn slippage_is_a_cost_not_baked_into_fill_prices() {
        let candles = vec![
            candle(0, 100, 101, 99, 100),
            candle(1, 100, 101, 99, 100),
            candle(2, 100, 110, 100, 110),
        ];
        let cfg = ExecutionConfig {
            slippage_ticks_per_side: 1,
            tick_size: Decimal::new(25, 2),
            ..ExecutionConfig::default()
        };

        let trades = run_setups(&candles, &[long_setup(EntryModel::NextBarOpen)], &cfg);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].entry, dv(100));
        assert_eq!(trades[0].tp, dv(110));
        assert_eq!(trades[0].slippage, Decimal::new(50, 2));
    }

    #[test]
    fn limit_fill_stops_out_on_same_bar_conservatively() {
        let candles = vec![candle(0, 100, 101, 99, 100), candle(1, 101, 102, 94, 96)];
        let setup = long_setup(EntryModel::LimitTouch {
            price: dv(100),
            expiry_bars: 2,
        });

        let trades = run_setups(&candles, &[setup], &ExecutionConfig::default());

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].result, TradeResult::Expense);
        assert_eq!(trades[0].entry, dv(100));
        assert_eq!(trades[0].tp, dv(95));
        assert_eq!(trades[0].close_time, candles[1].close_time);
    }
}
