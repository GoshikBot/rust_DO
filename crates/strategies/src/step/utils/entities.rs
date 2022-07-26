use std::fmt::{Display, Formatter};

pub mod angle;
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

pub type StrategyPerformance = f32;
