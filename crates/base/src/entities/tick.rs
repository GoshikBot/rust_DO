use crate::entities::MyFrom;
use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

pub type TickPrice = Decimal;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistoricalTickPrice {
    /// The high value of the candle that represents the tick.
    pub high: TickPrice,
    /// The low value of the candle that represents the tick.
    pub low: TickPrice,
    /// The close value of the candle that represents the tick.
    pub close: TickPrice,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UniversalTickPrice {
    Historical(HistoricalTickPrice),
    Realtime(TickPrice),
}

impl Default for UniversalTickPrice {
    fn default() -> Self {
        Self::Realtime(dec!(1.38000))
    }
}

impl From<HistoricalTickPrice> for UniversalTickPrice {
    fn from(price: HistoricalTickPrice) -> Self {
        UniversalTickPrice::Historical(price)
    }
}

impl From<TickPrice> for UniversalTickPrice {
    fn from(price: TickPrice) -> Self {
        UniversalTickPrice::Realtime(price)
    }
}

pub type TickId = String;
pub type TickTime = NaiveDateTime;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicTickProperties<P> {
    pub time: TickTime,
    pub ask: P,
    pub bid: P,
}

impl<P> MyFrom<BasicTickProperties<P>> for BasicTickProperties<UniversalTickPrice>
where
    P: Into<UniversalTickPrice>,
{
    fn my_from(properties: BasicTickProperties<P>) -> Self {
        Self {
            time: properties.time,
            ask: properties.ask.into(),
            bid: properties.bid.into(),
        }
    }
}

impl Default for BasicTickProperties<UniversalTickPrice> {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            ask: UniversalTickPrice::default(),
            bid: UniversalTickPrice::default(),
        }
    }
}

impl Default for BasicTickProperties<TickPrice> {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            ask: dec!(1.38),
            bid: dec!(1.37090),
        }
    }
}

impl Default for BasicTickProperties<HistoricalTickPrice> {
    fn default() -> Self {
        let historical_tick_price = HistoricalTickPrice {
            high: dec!(1.38),
            low: dec!(1.37090),
            close: dec!(1.37095),
        };

        Self {
            time: Utc::now().naive_utc(),
            ask: historical_tick_price,
            bid: historical_tick_price,
        }
    }
}
