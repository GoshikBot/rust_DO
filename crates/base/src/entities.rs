pub mod candle;
pub mod tick;

use crate::entities::candle::BasicCandleProperties;
pub use candle::{CandleEdgePrices, CandleMainProperties, CandleType};
use chrono::{DateTime, Duration, Utc};
use std::fmt::{Display, Formatter};
pub use tick::BasicTickProperties;

pub const LOT: u32 = 100_000;

pub const PRICE_DECIMAL_PLACES: u32 = 5;
pub const VOLUME_DECIMAL_PLACES: u32 = 2;

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

#[derive(Debug, Copy, Clone)]
pub enum Timeframe {
    Hour = 60,
    ThirtyMin = 30,
    FifteenMin = 15,
    OneMin = 1,
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
pub struct StrategyTimeframes {
    pub candle: Timeframe,
    pub tick: Timeframe,
}

#[derive(Debug, Clone)]
pub struct Item<I, P> {
    pub id: I,
    pub props: P,
}
