use chrono::{NaiveDateTime, Utc};

pub type CandleId = String;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CandleType {
    Green = 1,
    Red = -1,
    Neutral = 0,
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
