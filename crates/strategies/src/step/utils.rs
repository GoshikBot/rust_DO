use crate::step::utils::angle_utils::AngleUtils;
use crate::step::utils::backtesting_charts::{
    ChartIndex, ChartTraceEntity, StepBacktestingChartTraces,
};
use crate::step::utils::corridors::Corridors;
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::helpers::Helpers;
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::level_utils::LevelUtils;
use crate::step::utils::order_utils::OrderUtils;
use backtesting::trading_engine::TradingEngine;
use base::corridor::BasicCorridorUtils;
use base::entities::candle::{BasicCandleProperties, CandlePrice};
use base::entities::CandleType;
use base::helpers::{Holiday, NumberOfDaysToExclude};
use chrono::NaiveDateTime;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub mod angle_utils;
pub mod backtesting_charts;
pub mod corridors;
pub mod entities;
pub mod helpers;
pub mod level_conditions;
pub mod level_utils;
pub mod order_utils;
pub mod stores;
pub mod trading_limiter;

pub struct StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, E, D, X>
where
    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    Ang: AngleUtils,
    E: TradingEngine,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    helpers: PhantomData<Hel>,
    level_utils: PhantomData<LevUt>,
    level_conditions: PhantomData<LevCon>,
    order_utils: PhantomData<OrUt>,
    basic_corridor_utils: PhantomData<BCor>,
    corridors: PhantomData<Cor>,
    angle_utils: PhantomData<Ang>,
    pub trading_engine: E,
    pub add_entity_to_chart_traces: D,
    pub exclude_weekend_and_holidays: X,
}

impl<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, T, D, X>
    StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, T, D, X>
where
    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    Ang: AngleUtils,
    T: TradingEngine,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    pub fn new(
        add_entity_to_chart_traces: D,
        exclude_weekend_and_holidays: X,
        trading_engine: T,
    ) -> Self {
        Self {
            helpers: PhantomData,
            level_utils: PhantomData,
            level_conditions: PhantomData,
            order_utils: PhantomData,
            basic_corridor_utils: PhantomData,
            corridors: PhantomData,
            angle_utils: PhantomData,
            trading_engine,
            add_entity_to_chart_traces,
            exclude_weekend_and_holidays,
        }
    }
}

/// Determines the candle price to use for building the linear trading chart.
pub fn get_candle_leading_price(candle: &BasicCandleProperties) -> CandlePrice {
    match candle.r#type {
        CandleType::Green => candle.prices.high,
        CandleType::Red => candle.prices.low,
        CandleType::Neutral => {
            let candle_upper_part = candle.prices.high - candle.prices.close;
            let candle_lower_part = candle.prices.close - candle.prices.low;

            match candle_upper_part.cmp(&candle_lower_part) {
                Ordering::Less => candle.prices.low,
                Ordering::Greater => candle.prices.high,
                Ordering::Equal => candle.prices.high, // equally with low
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::entities::CandlePrices;
    use rust_decimal_macros::dec;

    #[test]
    #[allow(non_snake_case)]
    fn get_candle_leading_price__green_candle__should_return_high() {
        let candle = BasicCandleProperties {
            r#type: CandleType::Green,
            ..Default::default()
        };

        assert_eq!(get_candle_leading_price(&candle), candle.prices.high);
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_candle_leading_price__red_candle__should_return_low() {
        let candle = BasicCandleProperties {
            r#type: CandleType::Red,
            ..Default::default()
        };

        assert_eq!(get_candle_leading_price(&candle), candle.prices.low);
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_candle_leading_price__neutral_candle_upper_part_is_greater__should_return_high() {
        let candle = BasicCandleProperties {
            r#type: CandleType::Neutral,
            prices: CandlePrices {
                open: dec!(1.38000),
                high: dec!(1.38100),
                low: dec!(1.37950),
                close: dec!(1.38000),
            },
            ..Default::default()
        };

        assert_eq!(get_candle_leading_price(&candle), candle.prices.high);
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_candle_leading_price__neutral_candle_lower_part_is_greater__should_return_low() {
        let candle = BasicCandleProperties {
            r#type: CandleType::Neutral,
            prices: CandlePrices {
                open: dec!(1.38000),
                high: dec!(1.38050),
                low: dec!(1.37900),
                close: dec!(1.38000),
            },
            ..Default::default()
        };

        assert_eq!(get_candle_leading_price(&candle), candle.prices.low);
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_candle_leading_price__neutral_candle_upper_and_lower_parts_are_equal__should_return_high(
    ) {
        let candle = BasicCandleProperties {
            r#type: CandleType::Neutral,
            prices: CandlePrices {
                open: dec!(1.38000),
                high: dec!(1.38100),
                low: dec!(1.37900),
                close: dec!(1.38000),
            },
            ..Default::default()
        };

        assert_eq!(get_candle_leading_price(&candle), candle.prices.high);
    }
}
