use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub type TickPrice = Decimal;
pub type TickId = String;
pub type TickTime = NaiveDateTime;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicTickProperties {
    pub time: TickTime,
    pub ask: TickPrice,
    pub bid: TickPrice,
}

impl Default for BasicTickProperties {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            ask: dec!(1.38),
            bid: dec!(1.37090),
        }
    }
}
