use chrono::NaiveDateTime;

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

pub type CandleEdgePrice = f32;

#[derive(Debug, PartialEq, Clone)]
pub struct CandleEdgePrices {
    pub open: CandleEdgePrice,
    pub high: CandleEdgePrice,
    pub low: CandleEdgePrice,
    pub close: CandleEdgePrice,
}

