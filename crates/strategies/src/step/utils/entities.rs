use rust_decimal::Decimal;
use std::str::FromStr;

pub mod angle;
pub mod candle;
pub mod order;
pub mod params;
pub mod working_levels;

#[derive(Debug, Clone, Copy)]
pub enum Diff {
    Greater = 1,
    Less = -1,
}

#[derive(Debug)]
pub struct StrategySignals {
    pub no_trading_mode: bool,
    pub cancel_all_orders: bool,
}

pub type StrategyPerformance = Decimal;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Mode {
    Debug,
    Optimization,
}

impl FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Self::Debug),
            "optimization" => Ok(Self::Optimization),
            _ => anyhow::bail!("Invalid mode: {}", s),
        }
    }
}

pub const MODE_ENV: &str = "MODE";
