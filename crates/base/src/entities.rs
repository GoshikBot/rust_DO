pub mod candle;
pub mod tick;

use crate::entities::candle::BasicCandleProperties;
use anyhow::{bail, Result};
pub use candle::{CandleMainProperties, CandlePrices, CandleType};
use chrono::{DateTime, Duration, Utc};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
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
pub enum Tendency {
    Unknown = 0,
    Up = 1,
    Down = -1,
}

impl Default for Tendency {
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

impl FromStr for Timeframe {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "1m" => Ok(Self::OneMin),
            "15m" => Ok(Self::FifteenMin),
            "30m" => Ok(Self::ThirtyMin),
            "1h" => Ok(Self::Hour),
            _ => bail!("Invalid timeframe: {}", input),
        }
    }
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

pub const TARGET_LOGGER_ENV: &str = "TARGET_LOGGER";
pub const CANDLE_TIMEFRAME_ENV: &str = "CANDLE_TIMEFRAME";
pub const TICK_TIMEFRAME_ENV: &str = "TICK_TIMEFRAME";
