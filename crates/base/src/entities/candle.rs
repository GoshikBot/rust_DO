use chrono::{NaiveDateTime, Utc};
use float_cmp::{approx_eq, ApproxEq};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

pub type CandleId = String;

pub struct CandleOpenClose {
    pub open: CandlePrice,
    pub close: CandlePrice,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub enum CandleType {
    Green = 1,
    Red = -1,
    Neutral = 0,
}

impl From<CandleOpenClose> for CandleType {
    fn from(candle: CandleOpenClose) -> Self {
        let diff = candle.close - candle.open;

        match diff {
            n if n > dec!(0) => CandleType::Green,
            n if n == dec!(0) => CandleType::Neutral,
            n if n < dec!(0) => CandleType::Red,
            _ => unreachable!(),
        }
    }
}

pub type CandleSize = Decimal;
pub type CandleVolatility = u32;
pub type CandleTime = NaiveDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandleMainProperties {
    pub time: CandleTime,
    pub r#type: CandleType,
    pub size: CandleSize,
    pub volatility: CandleVolatility,
}

impl Default for CandleMainProperties {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(0.00100),
            volatility: 150,
        }
    }
}

pub type CandlePrice = Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandlePrices {
    pub open: CandlePrice,
    pub high: CandlePrice,
    pub low: CandlePrice,
    pub close: CandlePrice,
}

impl Default for CandlePrices {
    fn default() -> Self {
        Self {
            open: dec!(1.30945),
            high: dec!(1.31078),
            low: dec!(1.30939),
            close: dec!(1.31058),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct BasicCandleProperties {
    pub main_props: CandleMainProperties,
    pub edge_prices: CandlePrices,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_covert_candle_to_green_type() {
        let candle_open_close = CandleOpenClose {
            open: dec!(1.38),
            close: dec!(1.38001),
        };

        assert_eq!(CandleType::from(candle_open_close), CandleType::Green);
    }

    #[test]
    fn should_convert_candle_to_neutral_type() {
        let candle_open_close = CandleOpenClose {
            open: dec!(1.38),
            close: dec!(1.38),
        };

        assert_eq!(CandleType::from(candle_open_close), CandleType::Neutral);
    }

    #[test]
    fn should_convert_candle_to_red_type() {
        let candle_open_close = CandleOpenClose {
            open: dec!(1.38),
            close: dec!(1.37999),
        };

        assert_eq!(CandleType::from(candle_open_close), CandleType::Red);
    }
}
