pub mod candle;
pub mod tick;

use crate::entities::candle::BasicCandle;
pub use candle::{CandleBaseProperties, CandleEdgePrices, CandleType};
use chrono::{DateTime, Duration, Utc};
use std::fmt::{Display, Formatter};
pub use tick::BasicTick;

pub const LOT: i32 = 100_000;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Level {
    Min = -1,
    Max = 1,
}

#[derive(Debug, Copy, Clone)]
pub enum MovementType {
    Unknown = 0,
    Up = 1,
    Down = -1,
}

impl Default for MovementType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Edge {
    High = 1,
    Low = -1,
}

#[derive(Debug, PartialEq, Default)]
pub struct HistoricalData {
    pub candles: Vec<Option<BasicCandle>>,
    pub ticks: Vec<Option<BasicTick>>,
}

#[derive(Debug, Copy, Clone)]
pub enum Timeframe {
    Hour,
    ThirtyMin,
    FifteenMin,
    OneMin,
}

impl Display for Timeframe {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Timeframe::Hour => write!(f, "1h"),
            Timeframe::ThirtyMin => write!(f, "30m"),
            Timeframe::FifteenMin => write!(f, "15m"),
            Timeframe::OneMin => write!(f, "1m"),
        }
    }
}

#[derive(Debug)]
pub struct StrategyProperties {
    pub symbol: String,
    pub candle_timeframe: Timeframe,
    pub tick_timeframe: Timeframe,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
}
