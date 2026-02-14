use serde::Deserialize;

use super::candle_stick::CandleStick;

pub type BitgetKlinesItem = Vec<String>;
pub type MexcKlinesItem = Vec<String>;

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct BitgetResponse {
    pub code: String,
    pub msg: String,
    // pub request_time: String,
    pub data: Vec<BitgetKlinesItem>,
}

impl From<BitgetKlinesItem> for CandleStick {
    fn from(a: BitgetKlinesItem) -> CandleStick {
        let start: i64 = a.get(0).unwrap().parse().unwrap();
        CandleStick {
            open_time: start / 1000,
            open: super::decimal::DecimalVec::from_str(a.get(1).unwrap()),
            high: super::decimal::DecimalVec::from_str(a.get(2).unwrap()),
            low: super::decimal::DecimalVec::from_str(a.get(3).unwrap()),
            close: super::decimal::DecimalVec::from_str(a.get(4).unwrap()),
            close_time: start / 1000, // TODO: ???
        }
    }
}

// impl From<MexcKlinesItem> for CandleStick {
//     fn from(a: MexcKlinesItem) -> CandleStick {
//         let start: i64 = a.get(0).unwrap().parse().unwrap();
//         CandleStick {
//             open_time: start / 1000,
//             open: super::decimal::DecimalVec::from_str(a.get(1).unwrap()),
//             high: super::decimal::DecimalVec::from_str(a.get(2).unwrap()),
//             low: super::decimal::DecimalVec::from_str(a.get(3).unwrap()),
//             close: super::decimal::DecimalVec::from_str(a.get(4).unwrap()),
//             close_time: start / 1000, // TODO: ???
//         }
//     }
// }

// #[derive(Debug, Clone, Deserialize, serde::Serialize)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize))]
// pub struct BitgetKlinesItem {
//     pub item: Vec<String>,
// }
// startTime: 1724021850000
// response: 1724021760000
// =====> the last item is always returned from bitget api...
