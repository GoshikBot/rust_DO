use crate::step::utils::backtesting_charts::{BacktestingChartTracesModifier, ChartTracesModifier};
use crate::step::utils::helpers::Helpers;
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::level_utils::LevelUtils;
use crate::step::utils::order_utils::OrderUtils;
use backtesting::trading_engine::TradingEngine;

pub mod backtesting_charts;
pub mod entities;
pub mod helpers;
pub mod level_conditions;
pub mod level_utils;
pub mod order_utils;
pub mod stores;
pub mod trading_limiter;

pub struct StepBacktestingUtils<H, U, N, R, D, E>
where
    H: Helpers,
    U: LevelUtils,
    N: LevelConditions,
    R: OrderUtils,
    D: ChartTracesModifier,
    E: TradingEngine,
{
    pub helpers: H,
    pub level_utils: U,
    pub level_conditions: N,
    pub order_utils: R,
    pub chart_traces_modifier: D,
    pub trading_engine: E,
}
