use chrono::{NaiveDateTime, Utc};

pub type CandleId = String;

pub struct CandleOpenClose {
    pub open: CandleEdgePrice,
    pub close: CandleEdgePrice,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CandleType {
    Green = 1,
    Red = -1,
    Neutral = 0,
}

impl From<CandleOpenClose> for CandleType {
    fn from(candle: CandleOpenClose) -> Self {
        let diff = candle.close - candle.open;

        match diff {
            n if n > 0.0 => CandleType::Green,
            n if n == 0.0 => CandleType::Neutral,
            n if n < 0.0 => CandleType::Red,
            _ => unreachable!(),
        }
    }
}

pub type CandleSize = f32;
pub type CandleVolatility = f32;

#[derive(Debug, PartialEq, Clone)]
pub struct CandleBaseProperties {
    pub time: NaiveDateTime,
    pub r#type: CandleType,
    pub size: CandleSize,
    pub volatility: CandleVolatility,
}

impl Default for CandleBaseProperties {
    fn default() -> Self {
        Self {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: 0.00100,
            volatility: 150.0,
        }
    }
}

pub type CandleEdgePrice = f32;

#[derive(Debug, PartialEq, Clone)]
pub struct CandleEdgePrices {
    pub open: CandleEdgePrice,
    pub high: CandleEdgePrice,
    pub low: CandleEdgePrice,
    pub close: CandleEdgePrice,
}

impl Default for CandleEdgePrices {
    fn default() -> Self {
        Self {
            open: 1.30945,
            high: 1.31078,
            low: 1.30939,
            close: 1.31058,
        }
    }
}

#[derive(Debug)]
pub struct BasicCandle {
    pub base_properties: CandleBaseProperties,
    pub edge_prices: CandleEdgePrices,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_covert_candle_to_green_type() {
        let candle_open_close = CandleOpenClose {
            open: 1.38,
            close: 1.38001,
        };

        assert_eq!(CandleType::from(candle_open_close), CandleType::Green);
    }

    #[test]
    fn should_convert_candle_to_neutral_type() {
        let candle_open_close = CandleOpenClose {
            open: 1.38,
            close: 1.38,
        };

        assert_eq!(CandleType::from(candle_open_close), CandleType::Neutral);
    }

    #[test]
    fn should_convert_candle_to_red_type() {
        let candle_open_close = CandleOpenClose {
            open: 1.38,
            close: 1.37999,
        };

        assert_eq!(CandleType::from(candle_open_close), CandleType::Red);
    }
}
