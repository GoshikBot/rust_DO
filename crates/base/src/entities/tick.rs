use chrono::{NaiveDateTime, Utc};

pub type TickPrice = f32;
pub type TickId = String;

#[derive(Debug, PartialEq, Clone)]
pub struct BasicTick {
    pub time: NaiveDateTime,
    pub ask: TickPrice,
    pub bid: TickPrice,
}

impl Default for BasicTick {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            ask: 1.38,
            bid: 1.37090,
        }
    }
}
