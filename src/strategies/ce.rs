use chrono::Timelike;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;

#[derive(Clone, Debug)]
pub struct CeConfig {
    pub swing_lookback: usize,
    pub min_swing_points: Decimal,
    pub entry_at: Decimal,
    pub max_wait_bars: usize,
    pub stop_buffer_points: Decimal,
    pub base_rr: Decimal,
    pub trade_london_open: bool,
    pub trade_ny_am: bool,
    pub trade_ny_mid: bool,
    pub trade_power_hour: bool,
    pub use_trend_filter: bool,
    pub trend_lookback_days: usize,
    pub trend_strength_threshold: Decimal,
    pub use_vol_filter: bool,
    pub min_session_range: Decimal,
    pub max_session_range: Decimal,
    pub require_impulse_move: bool,
    pub use_dynamic_rr: bool,
    pub rr_trend_aligned: Decimal,
    pub rr_counter_trend: Decimal,
    pub max_hold_bars: usize,
    pub commission_round_trip_usd: Decimal,
    pub slippage_round_trip_usd: Decimal,
    pub require_rejection_confirm: bool,
}

impl Default for CeConfig {
    fn default() -> Self {
        Self {
            swing_lookback: 10,
            min_swing_points: Decimal::from(10),
            entry_at: Decimal::new(5, 1),
            max_wait_bars: 5,
            stop_buffer_points: Decimal::from(2),
            base_rr: Decimal::from(2),
            trade_london_open: false,
            trade_ny_am: true,
            trade_ny_mid: true,
            trade_power_hour: false,
            use_trend_filter: true,
            trend_lookback_days: 3,
            trend_strength_threshold: Decimal::new(6, 1),
            use_vol_filter: true,
            min_session_range: Decimal::from(15),
            max_session_range: Decimal::from(100),
            require_impulse_move: true,
            use_dynamic_rr: true,
            rr_trend_aligned: Decimal::new(25, 1),
            rr_counter_trend: Decimal::new(15, 1),
            max_hold_bars: 24,
            commission_round_trip_usd: Decimal::new(132, 2),
            slippage_round_trip_usd: Decimal::ONE,
            require_rejection_confirm: false,
        }
    }
}

pub struct CeStrategy {
    pub data_5m: Vec<CandleStick>,
    pub config: CeConfig,
}

#[derive(Clone, Copy)]
enum CeSide {
    Long,
    Short,
}

#[derive(Clone, Copy)]
struct Swing {
    side: CeSide,
    swing_low: Decimal,
    swing_high: Decimal,
}

impl CeStrategy {
    fn make_trade(&self, p: Position, close_time: i64, result: TradeResult) -> Trade {
        let mut t = Trade::from_position(p, close_time, result);
        t.commission = self.config.commission_round_trip_usd;
        t.slippage = self.config.slippage_round_trip_usd;
        t
    }

    fn killzone(&self, ts: i64) -> Option<&'static str> {
        let t = to_new_york_time(ts).time();
        let hm = (t.hour(), t.minute());
        let in_range = |h: u32, m: u32, h2: u32, m2: u32| hm >= (h, m) && hm <= (h2, m2);

        if self.config.trade_london_open && in_range(3, 0, 5, 0) {
            return Some("london_open");
        }
        if self.config.trade_ny_am && in_range(9, 30, 10, 30) {
            return Some("ny_am");
        }
        if self.config.trade_ny_mid && hm > (10, 30) && hm <= (14, 0) {
            return Some("ny_mid");
        }
        if self.config.trade_power_hour && hm > (14, 0) && hm <= (16, 0) {
            return Some("power_hour");
        }
        None
    }

    fn detect_swing(&self, day: &[CandleStick], i: usize) -> Option<Swing> {
        let lb = self.config.swing_lookback;
        if i < lb {
            return None;
        }
        let w = &day[i - lb..i];
        let mut hi = w[0].high.0;
        let mut lo = w[0].low.0;
        let mut hi_idx = 0usize;
        let mut lo_idx = 0usize;
        for (k, c) in w.iter().enumerate() {
            if c.high.0 > hi {
                hi = c.high.0;
                hi_idx = k;
            }
            if c.low.0 < lo {
                lo = c.low.0;
                lo_idx = k;
            }
        }
        let size = hi - lo;
        if size < self.config.min_swing_points {
            return None;
        }
        if lo_idx < hi_idx {
            Some(Swing {
                side: CeSide::Long,
                swing_low: lo,
                swing_high: hi,
            })
        } else if hi_idx < lo_idx {
            Some(Swing {
                side: CeSide::Short,
                swing_low: lo,
                swing_high: hi,
            })
        } else {
            None
        }
    }

    fn impulse_ok(&self, day: &[CandleStick], i: usize, side: CeSide) -> bool {
        if !self.config.require_impulse_move {
            return true;
        }
        let lb = self.config.swing_lookback;
        if i < lb + 1 {
            return false;
        }
        let w = &day[i - lb..i];
        let mut max_cons = 0usize;
        let mut cur = 0usize;
        for k in 1..w.len() {
            let up = w[k].close.0 > w[k - 1].close.0;
            let down = w[k].close.0 < w[k - 1].close.0;
            let match_impulse = match side {
                CeSide::Long => down,
                CeSide::Short => up,
            };
            if match_impulse {
                cur += 1;
                if cur > max_cons {
                    max_cons = cur;
                }
            } else if up || down {
                cur = 0;
            }
        }
        max_cons >= 3
    }

    fn dynamic_rr(&self, trend_aligned: bool, kz: &str) -> Decimal {
        if !self.config.use_dynamic_rr {
            return self.config.base_rr;
        }
        let rr = if trend_aligned {
            self.config.rr_trend_aligned
        } else {
            self.config.rr_counter_trend
        };
        let mul = match kz {
            "ny_am" => Decimal::new(11, 1),
            "power_hour" => Decimal::new(12, 1),
            _ => Decimal::ONE,
        };
        (rr * mul).min(Decimal::from(3))
    }
}

impl TradingModel for CeStrategy {
    fn execute(&self) -> BacktestResult {
        let mut trades = Vec::new();
        let mut day_bias: Vec<(chrono::NaiveDate, i8)> = Vec::new();
        {
            let mut i = 0usize;
            while i < self.data_5m.len() {
                let d = to_new_york_time(self.data_5m[i].open_time).date_naive();
                let start = i;
                while i < self.data_5m.len()
                    && to_new_york_time(self.data_5m[i].open_time).date_naive() == d
                {
                    i += 1;
                }
                let open = self.data_5m[start].open.0;
                let close = self.data_5m[i - 1].close.0;
                let b = if close > open {
                    1
                } else if close < open {
                    -1
                } else {
                    0
                };
                day_bias.push((d, b));
            }
        }
        let mut i = 0usize;
        while i < self.data_5m.len() {
            let day = to_new_york_time(self.data_5m[i].open_time).date_naive();
            let start = i;
            while i < self.data_5m.len()
                && to_new_york_time(self.data_5m[i].open_time).date_naive() == day
            {
                i += 1;
            }
            let day_slice = &self.data_5m[start..i];
            let day_index = day_bias.iter().position(|(d, _)| *d == day).unwrap_or(0);
            if day_slice.len() <= self.config.swing_lookback + 2 {
                continue;
            }

            let mut traded = false;
            for idx in self.config.swing_lookback..day_slice.len() {
                if traded {
                    break;
                }
                let candle = day_slice[idx];
                let kz = match self.killzone(candle.open_time) {
                    Some(v) => v,
                    None => continue,
                };

                let swing = match self.detect_swing(day_slice, idx) {
                    Some(v) => v,
                    None => continue,
                };

                if !self.impulse_ok(day_slice, idx, swing.side) {
                    continue;
                }

                if self.config.use_vol_filter {
                    let mut hi = day_slice[0].high.0;
                    let mut lo = day_slice[0].low.0;
                    for c in day_slice.iter().take(idx + 1) {
                        hi = hi.max(c.high.0);
                        lo = lo.min(c.low.0);
                    }
                    let range = hi - lo;
                    if range < self.config.min_session_range
                        || range > self.config.max_session_range
                    {
                        continue;
                    }
                }

                let trend_aligned = if self.config.use_trend_filter {
                    if day_index < self.config.trend_lookback_days {
                        true
                    } else {
                        let mut bull = 0usize;
                        let mut bear = 0usize;
                        for (_, b) in
                            &day_bias[day_index - self.config.trend_lookback_days..day_index]
                        {
                            if *b > 0 {
                                bull += 1;
                            } else if *b < 0 {
                                bear += 1;
                            }
                        }
                        let look =
                            Decimal::from_i32(self.config.trend_lookback_days as i32).unwrap();
                        let bp = Decimal::from_i32(bull as i32).unwrap() / look;
                        let sp = Decimal::from_i32(bear as i32).unwrap() / look;
                        match swing.side {
                            CeSide::Long => {
                                if sp >= self.config.trend_strength_threshold {
                                    continue;
                                }
                                bp >= self.config.trend_strength_threshold
                            }
                            CeSide::Short => {
                                if bp >= self.config.trend_strength_threshold {
                                    continue;
                                }
                                sp >= self.config.trend_strength_threshold
                            }
                        }
                    }
                } else {
                    true
                };
                let rr = self.dynamic_rr(trend_aligned, kz);
                let ce =
                    swing.swing_low + (swing.swing_high - swing.swing_low) * self.config.entry_at;

                let mut entered = None;
                for j in idx..(idx + self.config.max_wait_bars).min(day_slice.len()) {
                    if day_slice[j].low.0 <= ce && day_slice[j].high.0 >= ce {
                        if self.config.require_rejection_confirm {
                            let confirmed = match swing.side {
                                CeSide::Long => day_slice[j].close.0 > ce,
                                CeSide::Short => day_slice[j].close.0 < ce,
                            };
                            if !confirmed {
                                continue;
                            }
                        }
                        entered = Some(j);
                        break;
                    }
                }
                let Some(entry_idx) = entered else { continue };

                let (direction, sl, tp) = match swing.side {
                    CeSide::Long => {
                        let sl = swing.swing_low - self.config.stop_buffer_points;
                        let risk = ce - sl;
                        (PositionDirection::Long, sl, ce + risk * rr)
                    }
                    CeSide::Short => {
                        let sl = swing.swing_high + self.config.stop_buffer_points;
                        let risk = sl - ce;
                        (PositionDirection::Short, sl, ce - risk * rr)
                    }
                };

                let pos = Position {
                    direction,
                    open_time: day_slice[entry_idx].open_time,
                    entry: DecimalVec(ce),
                    sl: DecimalVec(sl),
                    tp: DecimalVec(tp),
                    at_break_even: false,
                };

                let mut closed = false;
                for k in (entry_idx + 1)..day_slice.len() {
                    let c = day_slice[k];
                    if k - entry_idx >= self.config.max_hold_bars {
                        let mut p = pos;
                        let exit = c.close.0;
                        let r = match direction {
                            PositionDirection::Long if exit > ce => {
                                p.tp = DecimalVec(exit);
                                TradeResult::Winner
                            }
                            PositionDirection::Short if exit < ce => {
                                p.tp = DecimalVec(exit);
                                TradeResult::Winner
                            }
                            _ => {
                                p.sl = DecimalVec(exit);
                                TradeResult::Expense
                            }
                        };
                        trades.push(self.make_trade(p, c.close_time, r));
                        closed = true;
                        break;
                    }

                    match direction {
                        PositionDirection::Long => {
                            if c.low.0 <= sl {
                                trades.push(self.make_trade(
                                    pos,
                                    c.close_time,
                                    TradeResult::Expense,
                                ));
                                closed = true;
                                break;
                            }
                            if c.high.0 >= tp {
                                trades.push(self.make_trade(
                                    pos,
                                    c.close_time,
                                    TradeResult::Winner,
                                ));
                                closed = true;
                                break;
                            }
                        }
                        PositionDirection::Short => {
                            if c.high.0 >= sl {
                                trades.push(self.make_trade(
                                    pos,
                                    c.close_time,
                                    TradeResult::Expense,
                                ));
                                closed = true;
                                break;
                            }
                            if c.low.0 <= tp {
                                trades.push(self.make_trade(
                                    pos,
                                    c.close_time,
                                    TradeResult::Winner,
                                ));
                                closed = true;
                                break;
                            }
                        }
                    }
                }

                if !closed {
                    let last = day_slice[day_slice.len() - 1];
                    let mut p = pos;
                    let exit = last.close.0;
                    let r = match direction {
                        PositionDirection::Long if exit > ce => {
                            p.tp = DecimalVec(exit);
                            TradeResult::Winner
                        }
                        PositionDirection::Short if exit < ce => {
                            p.tp = DecimalVec(exit);
                            TradeResult::Winner
                        }
                        _ => {
                            p.sl = DecimalVec(exit);
                            TradeResult::Expense
                        }
                    };
                    trades.push(self.make_trade(p, last.close_time, r));
                }

                traded = true;
            }
        }
        BacktestResult {
            trades,
            capital: Decimal::from_i32(1000).unwrap(),
        }
    }
}

pub fn resample_to_5m(candles: &[CandleStick]) -> Vec<CandleStick> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<i64, CandleStick> = BTreeMap::new();
    for c in candles {
        let b = c.open_time - (c.open_time % 300);
        map.entry(b)
            .and_modify(|acc| {
                if c.high.0 > acc.high.0 {
                    acc.high = c.high;
                }
                if c.low.0 < acc.low.0 {
                    acc.low = c.low;
                }
                acc.close = c.close;
                acc.close_time = c.close_time;
            })
            .or_insert(CandleStick {
                open_time: b,
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                close_time: c.close_time,
            });
    }
    map.into_values().collect()
}

pub fn score(result: &BacktestResult) -> (Decimal, Decimal, Decimal, usize) {
    let n = result.number_of_trades();
    let wins = result.result(TradeResult::Winner);
    let win = if n == 0 {
        Decimal::ZERO
    } else {
        Decimal::from_i32((wins * 100) as i32).unwrap() / Decimal::from_i32(n as i32).unwrap()
    };
    let mut gp = Decimal::ZERO;
    let mut gl = Decimal::ZERO;
    let r = Decimal::new(1, 2);
    let mut cap = Decimal::from(1000);
    for t in &result.trades {
        let ch = cap * r * t.gross_r() - t.total_costs();
        if ch > Decimal::ZERO {
            gp += ch;
        } else if ch < Decimal::ZERO {
            gl += -ch;
        }
        cap += ch;
    }
    let pf = if gl > Decimal::ZERO {
        gp / gl
    } else {
        Decimal::ZERO
    };
    let net_usd = cap - Decimal::from(1000);
    (net_usd, pf, win, n)
}
