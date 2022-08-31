use crate::step::utils::backtesting_charts::{ChartTraceEntity, StepBacktestingChartTraces};
use crate::step::utils::corridors::Corridors;
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::helpers::Helpers;
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::level_utils::LevelUtils;
use crate::step::utils::order_utils::OrderUtils;
use backtesting::trading_engine::TradingEngine;
use base::corridor::BasicCorridorUtils;
use base::helpers::{Holiday, NumberOfDaysToExclude};
use chrono::NaiveDateTime;
use std::marker::PhantomData;

pub mod backtesting_charts;
pub mod corridors;
pub mod entities;
pub mod helpers;
pub mod level_conditions;
pub mod level_utils;
pub mod order_utils;
pub mod stores;
pub mod trading_limiter;

pub struct StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, D, E, X>
where
    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, &StepBacktestingCandleProperties),
    E: TradingEngine,
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    helpers: PhantomData<Hel>,
    level_utils: PhantomData<LevUt>,
    level_conditions: PhantomData<LevCon>,
    order_utils: PhantomData<OrUt>,
    basic_corridor_utils: PhantomData<BCor>,
    corridors: PhantomData<Cor>,
    pub add_entity_to_chart_traces: D,
    pub trading_engine: E,
    pub exclude_weekend_and_holidays: X,
}

impl<Hel, LevUt, LevCon, OrUt, BCor, Cor, D, E, X>
    StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, D, E, X>
where
    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, &StepBacktestingCandleProperties),
    E: TradingEngine,
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    pub fn new(
        add_entity_to_chart_traces: D,
        trading_engine: E,
        exclude_weekend_and_holidays: X,
    ) -> Self {
        Self {
            helpers: PhantomData,
            level_utils: PhantomData,
            level_conditions: PhantomData,
            order_utils: PhantomData,
            basic_corridor_utils: PhantomData,
            corridors: PhantomData,
            add_entity_to_chart_traces,
            trading_engine,
            exclude_weekend_and_holidays,
        }
    }
}
