use crate::engine::entry_policies::resolve_entry_policy;
use crate::engine::types::{
    apply_entry_slippage, apply_exit_slippage, risk_amount, stop_hit,
    target_hit, EntryModel, EntryPolicy, ExecutionConfig, OpenPosition, SetupCandidate, StopModel,
    TargetModel, TrailingModel,
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

pub fn run_setups(candles: &[CandleStick], setups: &[SetupCandidate], cfg: &ExecutionConfig) -> Vec<Trade> {
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
    let mut pending_limit: Option<(SetupCandidate, usize, EntryPolicy)> = None;

    for (i, c) in candles.iter().copied().enumerate() {
        while setup_idx < setups.len() && setups[setup_idx].signal_index < i {
            metrics.skipped_other += 1;
            setup_idx += 1;
        }

        if open.is_none() {
            if let Some((candidate, expires_at, policy)) = pending_limit {
                if i > expires_at {
                    metrics.limit_orders_expired += 1;
                    pending_limit = None;
                } else {
                    let previous = if i > 0 { candles[i - 1] } else { c };
                    let price = resolve_entry_policy(policy, candidate.direction, c, previous);
                    if c.low <= price && c.high >= price {
                        open = build_position(candidate, DecimalVec(price.0), c.open_time, cfg);
                        if open.is_some() {
                            metrics.limit_orders_filled += 1;
                        }
                        pending_limit = None;
                    }
                }
            }

            while open.is_none() && setup_idx < setups.len() && setups[setup_idx].signal_index == i {
                let candidate = setups[setup_idx];
                match candidate.entry {
                    EntryModel::SignalClose | EntryModel::MarketClose => {
                        open = build_position(candidate, c.close, c.close_time, cfg);
                    }
                    EntryModel::NextBarOpen => {
                        if i + 1 < candles.len() {
                            let next = candles[i + 1];
                            open = build_position(candidate, next.open, next.open_time, cfg);
                        }
                    }
                    EntryModel::LimitTouch { expiry_bars, .. } => {
                        if let EntryModel::LimitTouch { price, .. } = candidate.entry {
                            metrics.limit_orders_placed += 1;
                            pending_limit = Some((
                                candidate,
                                i + expiry_bars,
                                EntryPolicy::Price(price),
                            ));
                        }
                    }
                    EntryModel::LimitByPolicy { policy, expiry_bars } => {
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
            apply_trailing(&mut p, c);

            if stop_hit(c, &p) {
                let stop_fill = gap_aware_stop_fill(c, &p, cfg);
                let result = if p.current_stop == p.entry {
                    TradeResult::BreakEven
                } else if (p.direction == crate::model::position_direction::PositionDirection::Long
                    && p.current_stop > p.entry)
                    || (p.direction == crate::model::position_direction::PositionDirection::Short
                        && p.current_stop < p.entry)
                {
                    TradeResult::Winner
                } else {
                    TradeResult::Expense
                };
                trades.push(build_trade(p, c.close_time, stop_fill, result, cfg));
                open = None;
            } else if target_hit(c, &p) {
                let tp_fill = apply_exit_slippage(p.direction, p.target, cfg);
                trades.push(build_trade(p, c.close_time, tp_fill, TradeResult::Winner, cfg));
                open = None;
            } else {
                open = Some(p);
            }
        }
    }

    if pending_limit.is_some() {
        metrics.limit_orders_expired += 1;
    }

    (trades, metrics)
}

fn build_position(
    candidate: SetupCandidate,
    raw_entry: DecimalVec,
    open_time: i64,
    cfg: &ExecutionConfig,
) -> Option<OpenPosition> {
    let entry = apply_entry_slippage(candidate.direction, raw_entry, cfg);
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
            crate::model::position_direction::PositionDirection::Long => DecimalVec(entry.0 + risk * r),
            crate::model::position_direction::PositionDirection::Short => DecimalVec(entry.0 - risk * r),
        },
        TargetModel::FixedPoints(points) => match candidate.direction {
            crate::model::position_direction::PositionDirection::Long => DecimalVec(entry.0 + points),
            crate::model::position_direction::PositionDirection::Short => DecimalVec(entry.0 - points),
        },
    };
    Some(OpenPosition {
        direction: candidate.direction,
        open_time,
        entry,
        stop,
        current_stop: stop,
        target,
        initial_risk: risk,
        trailing: candidate.trailing,
    })
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
    }
}

fn gap_aware_stop_fill(c: CandleStick, p: &OpenPosition, cfg: &ExecutionConfig) -> DecimalVec {
    match p.direction {
        crate::model::position_direction::PositionDirection::Long => {
            if c.open <= p.current_stop {
                apply_exit_slippage(p.direction, DecimalVec(c.open.0), cfg)
            } else {
                apply_exit_slippage(p.direction, p.current_stop, cfg)
            }
        }
        crate::model::position_direction::PositionDirection::Short => {
            if c.open >= p.current_stop {
                apply_exit_slippage(p.direction, DecimalVec(c.open.0), cfg)
            } else {
                apply_exit_slippage(p.direction, p.current_stop, cfg)
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
    let slippage = (cfg.tick_size * Decimal::from_i32(cfg.slippage_ticks_per_side).unwrap_or(Decimal::ZERO)).abs()
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
