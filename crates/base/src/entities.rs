pub mod candle;
pub mod order;
pub mod tick;

use crate::entities::order::OrderType;
use crate::helpers::Holiday;
use anyhow::Result;
pub use candle::{CandlePrices, CandleType};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
pub use tick::BasicTickProperties;

pub const LOT: u32 = 100_000;

pub const CANDLE_PRICE_DECIMAL_PLACES: u32 = 5;
pub const SIGNIFICANT_DECIMAL_PLACES: u32 = 2;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Level {
    Min = -1,
    Max = 1,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Tendency {
    #[default]
    Unknown = 0,
    Up = 1,
    Down = -1,
}

impl From<Level> for Tendency {
    fn from(level: Level) -> Self {
        match level {
            Level::Min => Self::Down,
            Level::Max => Self::Up,
        }
    }
}

impl From<Level> for OrderType {
    fn from(level: Level) -> Self {
        match level {
            Level::Min => OrderType::Sell,
            Level::Max => OrderType::Buy,
        }
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
    FiveMin = 5,
}

impl FromStr for Timeframe {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "1m" => Ok(Self::OneMin),
            "5m" => Ok(Self::FiveMin),
            "15m" => Ok(Self::FifteenMin),
            "30m" => Ok(Self::ThirtyMin),
            "1h" => Ok(Self::Hour),
            _ => anyhow::bail!("Invalid timeframe: {}", input),
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
            Timeframe::FiveMin => write!(f, "5m"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StrategyTimeframes {
    pub candle: Timeframe,
    pub tick: Timeframe,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item<I, P> {
    pub id: I,
    pub props: P,
}

pub const CANDLE_TIMEFRAME_ENV: &str = "CANDLE_TIMEFRAME";
pub const TICK_TIMEFRAME_ENV: &str = "TICK_TIMEFRAME";

pub const DEFAULT_HOLIDAYS: [Holiday; 2] =
    [Holiday { day: 25, month: 12 }, Holiday { day: 1, month: 1 }];

pub trait MyFrom<T> {
    fn my_from(other: T) -> Self;
}

impl<T, P> MyInto<T> for P
where
    T: MyFrom<P>,
{
    fn my_into(self) -> T {
        T::my_from(self)
    }
}

pub trait MyInto<T> {
    fn my_into(self) -> T;
}
