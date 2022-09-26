use anyhow::Context;
use anyhow::Result;
use backtesting::trading_engine::TradingEngine;
use backtesting::{BacktestingBalances, HistoricalData};
use base::corridor::BasicCorridorUtils;
use base::entities::candle::{BasicCandleProperties, CandlePrice};
use base::entities::{BasicTickProperties, StrategyTimeframes};
use base::helpers::{Holiday, NumberOfDaysToExclude};
use base::params::StrategyParams;
use base::stores::candle_store::BasicCandleStore;
use base::stores::order_store::BasicOrderStore;
use chrono::NaiveDateTime;
use rust_decimal_macros::dec;
use strategies::step::utils::angle_utils::AngleUtils;
use strategies::step::utils::backtesting_charts::{
    ChartIndex, ChartTraceEntity, StepBacktestingChartTraces,
};
use strategies::step::utils::corridors::Corridors;
use strategies::step::utils::entities::angle::BasicAngleProperties;
use strategies::step::utils::entities::candle::{
    StepBacktestingCandleProperties, StepCandleProperties,
};
use strategies::step::utils::entities::order::StepOrderProperties;
use strategies::step::utils::entities::params::{StepPointParam, StepRatioParam};
use strategies::step::utils::entities::working_levels::BacktestingWLProperties;
use strategies::step::utils::entities::{StrategyPerformance, StrategySignals};
use strategies::step::utils::helpers::Helpers;
use strategies::step::utils::level_conditions::LevelConditions;
use strategies::step::utils::level_utils::LevelUtils;
use strategies::step::utils::order_utils::OrderUtils;
use strategies::step::utils::stores::angle_store::StepAngleStore;
use strategies::step::utils::stores::tick_store::StepTickStore;
use strategies::step::utils::stores::working_level_store::StepWorkingLevelStore;
use strategies::step::utils::stores::{StepBacktestingMainStore, StepBacktestingStores};
use strategies::step::utils::trading_limiter::TradingLimiter;
use strategies::step::utils::{get_candle_leading_price, StepBacktestingUtils};

#[derive(Debug)]
struct Tick<'a, T> {
    index: usize,
    value: Option<&'a T>,
}

#[derive(Debug)]
struct Candle<'a, C> {
    index: usize,
    value: Option<&'a C>,
}

fn update_number_of_iterations_to_next_candle(
    number_of_iterations_to_next_candle: &mut u32,
    number_of_iterations_between_candles: u32,
) {
    if *number_of_iterations_to_next_candle == 0 {
        *number_of_iterations_to_next_candle = number_of_iterations_between_candles - 1;
    } else {
        *number_of_iterations_to_next_candle -= 1;
    }
}

fn strategy_performance(balances: &BacktestingBalances) -> StrategyPerformance {
    (balances.real - balances.initial) / balances.initial * dec!(100)
}

pub struct StepStrategyRunningConfig<'a, P, T, Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>
where
    P: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,

    T: StepBacktestingMainStore,

    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    Ang: AngleUtils,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    E: TradingEngine,
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    pub timeframes: StrategyTimeframes,
    pub stores: &'a mut StepBacktestingStores<T>,
    pub utils: &'a StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>,
    pub params: &'a P,
}

pub fn loop_through_historical_data<P, L, T, Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X, I>(
    historical_data: &HistoricalData<StepCandleProperties, BasicTickProperties>,
    strategy_config: StepStrategyRunningConfig<
        P,
        T,
        Hel,
        LevUt,
        LevCon,
        OrUt,
        BCor,
        Cor,
        Ang,
        D,
        E,
        X,
    >,
    trading_limiter: &L,
    get_candle_leading_price: &impl Fn(&BasicCandleProperties) -> CandlePrice,
    run_iteration: &I,
) -> Result<StrategyPerformance>
where
    P: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    L: TradingLimiter,

    T: StepBacktestingMainStore,

    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    Ang: AngleUtils,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    E: TradingEngine,
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,

    I: Fn(
        BasicTickProperties,
        Option<StepBacktestingCandleProperties>,
        StrategySignals,
        &mut StepBacktestingStores<T>,
        &StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>,
        &P,
    ) -> Result<()>,
{
    let mut current_tick = Tick {
        index: 0,
        value: historical_data
            .ticks
            .get(0)
            .context("no first tick")?
            .as_ref(),
    };

    let mut current_candle = Candle {
        index: 0,
        value: historical_data
            .candles
            .get(0)
            .context("no first candle")?
            .as_ref(),
    };

    let mut first_candle = true;

    let mut new_candle_appeared = false;

    let mut no_trading_mode = false;
    let mut cancel_all_orders = false;

    let number_of_iterations_between_candles =
        strategy_config.timeframes.candle as u32 / strategy_config.timeframes.tick as u32;
    let mut number_of_iterations_to_next_candle = number_of_iterations_between_candles - 1;

    loop {
        if let Some(current_tick) = current_tick.value {
            if no_trading_mode {
                if trading_limiter.allow_trading(current_tick) {
                    no_trading_mode = false;
                }
            } else if trading_limiter.forbid_trading(current_tick) {
                no_trading_mode = true;
                cancel_all_orders = true;
            }

            // run iteration only if a tick exists
            run_iteration(
                current_tick.clone(),
                if new_candle_appeared {
                    current_candle
                        .value
                        .map(|candle_props| StepBacktestingCandleProperties {
                            step_common: candle_props.clone(),
                            chart_index: current_candle.index,
                        })
                } else {
                    None
                },
                StrategySignals {
                    no_trading_mode,
                    cancel_all_orders,
                },
                strategy_config.stores,
                strategy_config.utils,
                strategy_config.params,
            )?;

            if cancel_all_orders {
                cancel_all_orders = false;
            }
        }

        if new_candle_appeared {
            new_candle_appeared = false;
        }

        update_number_of_iterations_to_next_candle(
            &mut number_of_iterations_to_next_candle,
            number_of_iterations_between_candles,
        );

        // the moment to update the current candle
        if number_of_iterations_to_next_candle == 0 {
            if !first_candle {
                let new_candle_value = historical_data.candles.get(current_candle.index + 1);
                match new_candle_value {
                    Some(new_candle) => {
                        current_candle = Candle {
                            index: current_candle.index + 1,
                            value: new_candle.as_ref(),
                        };
                    }
                    None => break,
                }
            } else {
                first_candle = false;
            }

            new_candle_appeared = true;
        }

        // the moment to update the current tick
        let new_tick_value = historical_data.ticks.get(current_tick.index + 1);
        match new_tick_value {
            None => break,
            Some(next_tick_value) => {
                current_tick = Tick {
                    index: current_tick.index + 1,
                    value: next_tick_value.as_ref(),
                };
            }
        }
    }

    Ok(strategy_performance(
        &strategy_config.stores.config.trading_engine.balances,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use backtesting::trading_engine::TradingEngine;
    use backtesting::{BacktestingTradingEngineConfig, Balance, ClosePositionBy, OpenPositionBy};
    use base::entities::candle::{CandleId, CandleVolatility};
    use base::entities::order::{BasicOrderProperties, OrderId, OrderPrice, OrderType};
    use base::entities::tick::{TickPrice, TickTime};
    use base::entities::{Item, Timeframe};
    use base::helpers::{Holiday, NumberOfDaysToExclude};
    use base::notifier::NotificationQueue;
    use base::params::ParamValue;
    use chrono::{NaiveDateTime, Timelike};
    use float_cmp::approx_eq;
    use rust_decimal_macros::dec;
    use std::fmt::Debug;
    use strategies::step::utils::angle_utils::ExistingDiffs;
    use strategies::step::utils::backtesting_charts::{
        ChartTraceEntity, StepBacktestingChartTraces,
    };
    use strategies::step::utils::corridors::{Corridors, UpdateCorridorsNearWorkingLevelsUtils};
    use strategies::step::utils::entities::angle::{AngleId, FullAngleProperties};
    use strategies::step::utils::entities::working_levels::{
        BasicWLProperties, CorridorType, LevelTime, WLId, WLMaxCrossingValue, WLPrice,
    };
    use strategies::step::utils::entities::{
        Diff, MaxMinAngles, StatisticsChartsNotifier, StatisticsNotifier,
    };
    use strategies::step::utils::helpers::HelpersImpl;
    use strategies::step::utils::level_conditions::MinAmountOfCandles;
    use strategies::step::utils::level_utils::{
        RemoveInvalidWorkingLevelsUtils, UpdateTendencyAndCreateWorkingLevelUtils,
    };
    use strategies::step::utils::order_utils::{
        OrderUtilsImpl, UpdateOrdersBacktestingStores, UpdateOrdersBacktestingUtils,
    };
    use strategies::step::utils::stores::candle_store::StepCandleStore;
    use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use strategies::step::utils::stores::{
        StepBacktestingConfig, StepBacktestingMainStore, StepConfig,
    };

    const HOUR_TO_FORBID_TRADING: u8 = 23;
    const HOURS_TO_FORBID_TRADING: [u8; 3] = [23, 0, 1];

    #[derive(Default)]
    struct TestTradingLimiter;

    impl TestTradingLimiter {
        fn new() -> Self {
            Default::default()
        }
    }

    impl TradingLimiter for TestTradingLimiter {
        fn forbid_trading(&self, current_tick: &BasicTickProperties) -> bool {
            if current_tick.time.time().hour() as u8 == HOUR_TO_FORBID_TRADING {
                return true;
            }

            false
        }

        fn allow_trading(&self, current_tick: &BasicTickProperties) -> bool {
            if HOURS_TO_FORBID_TRADING.contains(&(current_tick.time.time().hour() as u8)) {
                return false;
            }

            true
        }
    }

    #[derive(Default)]
    struct TestStrategyParams;

    impl TestStrategyParams {
        fn new() -> Self {
            Default::default()
        }
    }

    impl StrategyParams for TestStrategyParams {
        type PointParam = StepPointParam;
        type RatioParam = StepRatioParam;

        fn get_point_param_value(&self, _name: Self::PointParam) -> ParamValue {
            todo!()
        }

        fn get_ratio_param_value(
            &self,
            _name: Self::RatioParam,
            _volatility: CandleVolatility,
        ) -> ParamValue {
            todo!()
        }
    }

    #[derive(Default)]
    struct TestHelpersImpl;

    impl Helpers for TestHelpersImpl {}

    #[derive(Default)]
    struct TestLevelUtilsImpl;

    impl LevelUtils for TestLevelUtilsImpl {
        fn get_crossed_level<W>(
            current_tick_price: TickPrice,
            created_working_levels: &[Item<WLId, W>],
        ) -> Option<&Item<WLId, W>>
        where
            W: AsRef<BasicWLProperties>,
        {
            unimplemented!()
        }

        fn remove_active_working_levels_with_closed_orders<O>(
            working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
        ) -> Result<()>
        where
            O: Into<StepOrderProperties>,
        {
            unimplemented!()
        }

        fn update_max_crossing_value_of_active_levels<T>(
            working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
            current_tick_price: TickPrice,
        ) -> Result<()>
        where
            T: Into<BasicWLProperties>,
        {
            unimplemented!()
        }

        fn remove_invalid_working_levels<W, A, D, M, C, E, T, N, O>(
            current_tick: &BasicTickProperties,
            current_volatility: CandleVolatility,
            utils: RemoveInvalidWorkingLevelsUtils<W, A, D, M, C, E, T, O>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
            entity: StatisticsNotifier<N>,
        ) -> Result<()>
        where
            T: AsRef<BasicWLProperties>,
            O: AsRef<BasicOrderProperties>,
            W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
            A: Fn(&[O]) -> bool,
            D: Fn(WLPrice, TickPrice, ParamValue) -> bool,
            M: Fn(LevelTime, TickTime, ParamValue, &E) -> bool,
            C: Fn(&T, Option<WLMaxCrossingValue>, ParamValue, TickPrice) -> bool,
            E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
            N: NotificationQueue,
        {
            unimplemented!()
        }

        fn move_take_profits<W>(
            working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = W>,
            distance_from_level_for_signaling_of_moving_take_profits: ParamValue,
            distance_to_move_take_profits: ParamValue,
            current_tick_price: TickPrice,
        ) -> Result<()>
        where
            W: Into<BasicWLProperties>,
        {
            unimplemented!()
        }

        fn update_tendency_and_get_instruction_to_create_new_working_level<
            S,
            D,
            A,
            C,
            N,
            H,
            B,
            P,
            M,
            K,
            X,
            L,
        >(
            config: &mut StepConfig,
            store: &mut S,
            utils: UpdateTendencyAndCreateWorkingLevelUtils<D, A, C, S, B, P, M, K, X, L>,
            statistics_charts_notifier: StatisticsChartsNotifier<N, H>,
            crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
            current_candle: &Item<CandleId, C>,
            params: &M,
        ) -> Result<bool>
        where
            S: StepAngleStore<AngleProperties = A, CandleProperties = C>
                + StepCandleStore<CandleProperties = C>
                + StepWorkingLevelStore<WorkingLevelProperties = K>,
            D: Fn(&str, Option<&str>, bool, bool) -> bool,
            A: AsRef<BasicAngleProperties> + Debug,
            C: AsRef<StepCandleProperties> + Debug + PartialEq,
            N: NotificationQueue,
            H: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
            B: Fn(
                &Item<AngleId, FullAngleProperties<A, C>>,
                &[Item<CandleId, C>],
                &S,
                ParamValue,
            ) -> Result<bool>,
            M: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
            P: Fn(
                &Item<AngleId, FullAngleProperties<A, C>>,
                &Item<CandleId, C>,
                &S,
                &M,
            ) -> Result<bool>,
            K: AsRef<BasicWLProperties>,
            X: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S) -> Result<bool>,
            L: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S, ParamValue) -> Result<bool>,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestLevelConditionsImpl;

    impl LevelConditions for TestLevelConditionsImpl {
        fn level_exceeds_amount_of_candles_in_corridor(
            level_id: &str,
            working_level_store: &impl StepWorkingLevelStore,
            corridor_type: CorridorType,
            min_amount_of_candles: MinAmountOfCandles,
        ) -> Result<bool> {
            unimplemented!()
        }

        fn price_is_beyond_stop_loss(
            current_tick_price: TickPrice,
            stop_loss_price: OrderPrice,
            working_level_type: OrderType,
        ) -> bool {
            unimplemented!()
        }

        fn level_expired_by_distance(
            level_price: WLPrice,
            current_tick_price: TickPrice,
            distance_from_level_for_its_deletion: ParamValue,
        ) -> bool {
            unimplemented!()
        }

        fn level_expired_by_time(
            level_time: LevelTime,
            current_tick_time: TickTime,
            level_expiration: ParamValue,
            exclude_weekend_and_holidays: &impl Fn(
                NaiveDateTime,
                NaiveDateTime,
                &[Holiday],
            ) -> NumberOfDaysToExclude,
        ) -> bool {
            unimplemented!()
        }

        fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            level: &impl AsRef<BasicWLProperties>,
            max_crossing_value: Option<WLMaxCrossingValue>,
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
            current_tick_price: TickPrice,
        ) -> bool {
            unimplemented!()
        }

        fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
            unimplemented!()
        }

        fn is_second_level_after_bargaining_tendency_change(
            crossed_angle: &str,
            tendency_change_angle: Option<&str>,
            last_tendency_changed_on_crossing_bargaining_corridor: bool,
            second_level_after_bargaining_tendency_change_is_created: bool,
        ) -> bool {
            unimplemented!()
        }

        fn level_comes_out_of_bargaining_corridor<A, C>(
            crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
            general_corridor: &[Item<CandleId, C>],
            angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
            min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamValue,
        ) -> Result<bool>
        where
            A: AsRef<BasicAngleProperties> + Debug,
            C: AsRef<StepCandleProperties> + Debug + PartialEq,
        {
            unimplemented!()
        }

        fn appropriate_working_level<A, C>(
            crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
            current_candle: &Item<CandleId, C>,
            angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        ) -> Result<bool>
        where
            A: AsRef<BasicAngleProperties> + Debug,
            C: AsRef<StepCandleProperties> + Debug,
        {
            unimplemented!()
        }

        fn working_level_exists<A, C, W>(
            crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
            working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        ) -> Result<bool>
        where
            A: AsRef<BasicAngleProperties> + Debug,
            C: AsRef<StepCandleProperties> + Debug,
            W: AsRef<BasicWLProperties>,
        {
            unimplemented!()
        }

        fn working_level_is_close_to_another_one<A, C, W>(
            crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
            working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
            distance_defining_nearby_levels_of_the_same_type: ParamValue,
        ) -> Result<bool>
        where
            A: AsRef<BasicAngleProperties> + Debug,
            C: AsRef<StepCandleProperties> + Debug,
            W: AsRef<BasicWLProperties> + Debug,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestOrderUtilsImpl;

    impl OrderUtils for TestOrderUtilsImpl {
        fn get_new_chain_of_orders<W>(
            level: &Item<WLId, W>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
            current_volatility: CandleVolatility,
            current_balance: Balance,
        ) -> Result<Vec<StepOrderProperties>>
        where
            W: AsRef<BasicWLProperties>,
        {
            unimplemented!()
        }

        fn update_orders_backtesting<T, C, R, W, P>(
            current_tick: &BasicTickProperties,
            current_candle: &StepBacktestingCandleProperties,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
            stores: UpdateOrdersBacktestingStores<W>,
            utils: UpdateOrdersBacktestingUtils<T, C, R, W, P>,
            no_trading_mode: bool,
        ) -> Result<()>
        where
            W: BasicOrderStore<OrderProperties = StepOrderProperties>
                + StepWorkingLevelStore<WorkingLevelProperties = BacktestingWLProperties>,
            T: TradingEngine,
            C: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
            R: Fn(&str, &W, CorridorType, MinAmountOfCandles) -> Result<bool>,
            P: Fn(TickPrice, OrderPrice, OrderType) -> bool,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestCorridorsImpl;

    impl Corridors for TestCorridorsImpl {
        fn update_corridors_near_working_levels<W, O, C, L, N, R, A>(
            working_level_store: &mut impl StepWorkingLevelStore<
                WorkingLevelProperties = W,
                OrderProperties = O,
                CandleProperties = C,
            >,
            current_candle: &Item<CandleId, C>,
            utils: UpdateCorridorsNearWorkingLevelsUtils<C, O, L, N, R, A>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        ) -> Result<()>
        where
            W: Into<BasicWLProperties>,
            O: AsRef<BasicOrderProperties>,
            C: AsRef<BasicCandleProperties> + Debug,
            L: Fn(&C) -> bool,
            N: Fn(&C, &C, ParamValue) -> bool,
            R: Fn(
                &[Item<CandleId, C>],
                &Item<CandleId, C>,
                ParamValue,
                &dyn Fn(&C) -> bool,
                &dyn Fn(&C, &C, ParamValue) -> bool,
            ) -> Option<Vec<Item<CandleId, C>>>,
            A: Fn(&[O]) -> bool,
        {
            unimplemented!()
        }
    }

    struct TestBasicCorridorUtilsImpl;

    impl BasicCorridorUtils for TestBasicCorridorUtilsImpl {
        fn candle_can_be_corridor_leader(
            candle_properties: &impl AsRef<BasicCandleProperties>,
        ) -> bool {
            unimplemented!()
        }

        fn candle_is_in_corridor<C>(
            candle: &C,
            leading_candle: &C,
            max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
        ) -> bool
        where
            C: AsRef<BasicCandleProperties>,
        {
            unimplemented!()
        }

        fn crop_corridor_to_closest_leader<C>(
            corridor: &[Item<CandleId, C>],
            new_candle: &Item<CandleId, C>,
            max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
            candle_can_be_corridor_leader: &dyn Fn(&C) -> bool,
            is_in_corridor: &dyn Fn(&C, &C, ParamValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>
        where
            C: AsRef<BasicCandleProperties> + Clone,
        {
            unimplemented!()
        }
    }

    struct TestAngleUtilsImpl;

    impl AngleUtils for TestAngleUtilsImpl {
        fn get_diff_between_current_and_previous_candles<C>(
            current_candle_props: &C,
            previous_candle_props: &C,
        ) -> Diff
        where
            C: AsRef<StepCandleProperties>,
        {
            unimplemented!()
        }

        fn get_new_angle<C, A>(
            previous_candle: &Item<CandleId, C>,
            diffs: ExistingDiffs,
            angles: MaxMinAngles<A, C>,
            min_distance_between_max_min_angles: ParamValue,
            max_distance_between_max_min_angles: ParamValue,
        ) -> Option<FullAngleProperties<BasicAngleProperties, C>>
        where
            C: AsRef<StepCandleProperties> + Debug + Clone,
            A: AsRef<BasicAngleProperties> + Debug + Clone,
        {
            unimplemented!()
        }

        fn update_angles<A, C>(
            new_angle: FullAngleProperties<A, C>,
            general_corridor: &[Item<CandleId, C>],
            angle_store: &mut impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        ) -> Result<()>
        where
            A: AsRef<BasicAngleProperties> + Debug + Clone,
            C: AsRef<StepCandleProperties> + Debug + Clone + PartialEq,
        {
            unimplemented!()
        }

        fn get_crossed_angle<'a, A, C>(
            angles: MaxMinAngles<'a, A, C>,
            current_candle: &C,
        ) -> Option<&'a Item<AngleId, FullAngleProperties<A, C>>>
        where
            C: AsRef<StepCandleProperties> + Debug + Clone,
            A: AsRef<BasicAngleProperties> + Debug + Clone,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestTradingEngineImpl;

    impl TradingEngine for TestTradingEngineImpl {
        fn open_position<O>(
            &self,
            _order: &Item<OrderId, O>,
            _by: OpenPositionBy,
            _order_store: &mut impl BasicOrderStore,
            _trading_config: &mut BacktestingTradingEngineConfig,
        ) -> Result<()>
        where
            O: Into<BasicOrderProperties> + Clone,
        {
            unimplemented!()
        }

        fn close_position<O>(
            &self,
            _order: &Item<OrderId, O>,
            _by: ClosePositionBy,
            _order_store: &mut impl BasicOrderStore<OrderProperties = O>,
            _trading_config: &mut BacktestingTradingEngineConfig,
        ) -> Result<()>
        where
            O: Into<BasicOrderProperties> + Clone,
        {
            unimplemented!()
        }
    }

    #[test]
    fn loop_through_historical_data_proper_params_get_correct_performance() {
        let historical_data = HistoricalData {
            candles: vec![
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 18:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                None,
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 20:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 21:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 22:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 23:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 00:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 01:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                None,
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 03:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 04:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
                Some(StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 05:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                }),
            ],
            ticks: vec![
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 18:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 19:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 19:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 20:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 20:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 21:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                None,
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 22:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 22:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 23:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 23:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 00:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 00:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 01:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                None,
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 02:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 02:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 03:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 03:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 04:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 04:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 05:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 05:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 06:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: dec!(0),
                    bid: dec!(0),
                }),
            ],
        };

        let in_memory_store = InMemoryStepBacktestingStore::new();

        let mut step_stores = StepBacktestingStores {
            main: InMemoryStepBacktestingStore::new(),
            config: StepBacktestingConfig::default(10),
            statistics: Default::default(),
        };

        let step_params = TestStrategyParams::new();

        let trading_limiter = TestTradingLimiter::new();

        let exclude_weekend_and_holidays =
            |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 0;

        fn add_entity_to_chart_traces(
            _entity: ChartTraceEntity,
            _chart_traces: &mut StepBacktestingChartTraces,
            _current_candle_index: ChartIndex,
        ) {
            unimplemented!()
        }

        let utils: StepBacktestingUtils<
            TestHelpersImpl,
            TestLevelUtilsImpl,
            TestLevelConditionsImpl,
            TestOrderUtilsImpl,
            TestBasicCorridorUtilsImpl,
            TestCorridorsImpl,
            TestAngleUtilsImpl,
            _,
            _,
            _,
        > = StepBacktestingUtils::new(
            add_entity_to_chart_traces,
            TestTradingEngineImpl::default(),
            exclude_weekend_and_holidays,
        );

        let strategy_config = StepStrategyRunningConfig {
            timeframes: StrategyTimeframes {
                candle: Timeframe::Hour,
                tick: Timeframe::ThirtyMin,
            },
            stores: &mut step_stores,
            utils: &utils,
            params: &step_params,
        };

        fn run_iteration<T, Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>(
            new_tick_props: BasicTickProperties,
            new_candle_props: Option<StepBacktestingCandleProperties>,
            signals: StrategySignals,
            stores: &mut StepBacktestingStores<T>,
            utils: &StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        ) -> Result<()>
        where
            T: StepBacktestingMainStore,

            Hel: Helpers,
            LevUt: LevelUtils,
            LevCon: LevelConditions,
            OrUt: OrderUtils,
            BCor: BasicCorridorUtils,
            Cor: Corridors,
            Ang: AngleUtils,
            D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
            E: TradingEngine,
            X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
        {
            if signals.cancel_all_orders {
                stores.config.trading_engine.balances.real -= dec!(50.0);
            }

            if !signals.no_trading_mode {
                stores.config.trading_engine.balances.real += dec!(10.0);

                if new_candle_props.is_some() {
                    stores.config.trading_engine.balances.real += dec!(20.0);
                }
            }

            Ok(())
        }

        let get_candle_leading_price = |candle: &BasicCandleProperties| candle.prices.high;

        let strategy_performance = loop_through_historical_data(
            &historical_data,
            strategy_config,
            &trading_limiter,
            &get_candle_leading_price,
            &run_iteration,
        )
        .unwrap();

        assert_eq!(strategy_performance, dec!(2.6));
        assert_eq!(
            step_stores.config.trading_engine.balances.real,
            dec!(10_260)
        );
    }
}
