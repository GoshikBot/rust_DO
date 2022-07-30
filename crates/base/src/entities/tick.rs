use chrono::{NaiveDateTime, Utc};
use float_cmp::approx_eq;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub type TickPrice = Decimal;
pub type TickId = String;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicTickProperties {
    pub time: NaiveDateTime,
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
