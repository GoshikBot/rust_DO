use chrono::{NaiveDateTime, Utc};
use float_cmp::approx_eq;

pub type TickPrice = f32;
pub type TickId = String;

#[derive(Debug, Clone)]
pub struct BasicTickProperties {
    pub time: NaiveDateTime,
    pub ask: TickPrice,
    pub bid: TickPrice,
}

impl Default for BasicTickProperties {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            ask: 1.38,
            bid: 1.37090,
        }
    }
}

impl PartialEq for BasicTickProperties {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
            && approx_eq!(TickPrice, self.ask, other.ask)
            && approx_eq!(TickPrice, self.bid, other.bid)
    }
}
