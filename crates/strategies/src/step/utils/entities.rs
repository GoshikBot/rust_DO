use crate::step::utils::backtesting_charts::{
    ChartIndex, ChartTraceEntity, StepBacktestingChartTraces,
};
use crate::step::utils::entities::angle::{AngleId, BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::{StepBacktestingCandleProperties, StepCandleProperties};
use crate::step::utils::stores::StepBacktestingStatistics;
use base::entities::Item;
use base::notifier::{Message, NotificationQueue};
use rust_decimal::Decimal;
use std::fmt::Debug;
use std::str::FromStr;

pub mod angle;
pub mod candle;
pub mod order;
pub mod params;
pub mod working_levels;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Diff {
    Greater = 1,
    Less = -1,
}

#[derive(Debug)]
pub struct StrategySignals {
    pub no_trading_mode: bool,
    pub close_all_orders: bool,
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

pub enum StatisticsChartsNotifier<'a, N, H>
where
    N: NotificationQueue,
    H: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
{
    Backtesting {
        statistics: &'a mut StepBacktestingStatistics,
        add_entity_to_chart_traces: &'a H,
        chart_traces: &'a mut StepBacktestingChartTraces,
        current_candle_chart_index: ChartIndex,
        crossed_angle_candle_chart_index: ChartIndex,
    },
    Realtime(&'a N),
}

pub enum StatisticsNotifier<'a, N>
where
    N: NotificationQueue,
{
    Backtesting(&'a mut StepBacktestingStatistics),
    Realtime(&'a N),
}

pub struct FakeBacktestingNotificationQueue;

impl NotificationQueue for FakeBacktestingNotificationQueue {
    fn send_message(&self, _message: Message) -> anyhow::Result<()> {
        unreachable!()
    }
}

#[derive(Debug, Clone)]
pub struct MaxMinAngles<'a, A, C>
where
    C: AsRef<StepCandleProperties> + Debug + Clone,
    A: AsRef<BasicAngleProperties> + Debug + Clone,
{
    pub max_angle: &'a Option<Item<AngleId, FullAngleProperties<A, C>>>,
    pub min_angle: &'a Option<Item<AngleId, FullAngleProperties<A, C>>>,
}

impl<'a, A, C> Copy for MaxMinAngles<'a, A, C>
where
    A: AsRef<BasicAngleProperties> + Debug + Clone,
    C: AsRef<StepCandleProperties> + Debug + Clone,
{
}
