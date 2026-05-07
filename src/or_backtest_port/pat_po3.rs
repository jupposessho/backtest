use chrono::NaiveTime;

use super::{Config, EntryFillMode, ReversalSpec};

pub(super) fn reversal_spec() -> ReversalSpec {
    ReversalSpec {
        pre_start: NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        pre_end: NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
        trade_start: NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
        trade_end: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        allow_second_trade: false,
        sweep_tolerance_pct: 0.001,
    }
}

pub(super) fn apply_preset(cfg: &mut Config) {
    cfg.entry_fill_mode = EntryFillMode::Close;
    cfg.position_sizing_type = "Fixed USD Risk".to_string();
    cfg.fixed_usd_risk = 150.0;
    cfg.fixed_contracts = 1.0;
    cfg.breakout_candles = 6;
    cfg.sl_value = 0.8;
    cfg.tp_value = 2.0;
    cfg.enable_second_chance = false;
    cfg.rth_start = "00:00".to_string();
    cfg.rth_end = "23:59".to_string();
}
