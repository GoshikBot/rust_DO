use anyhow::Context;
use anyhow::Result;
use backtesting::trading_engine::TradingEngine;
use backtesting::{BacktestingBalances, HistoricalData};
use base::entities::candle::BasicCandleProperties;
use base::entities::{BasicTickProperties, StrategyTimeframes};
use base::params::StrategyParams;
use base::stores::candle_store::BasicCandleStore;
use base::stores::order_store::BasicOrderStore;
use rust_decimal_macros::dec;
use strategies::step::step_backtesting::RunStepBacktestingIteration;
use strategies::step::utils::backtesting_charts::ChartTracesModifier;
use strategies::step::utils::entities::angle::BasicAngleProperties;
use strategies::step::utils::entities::candle::StepBacktestingCandleProperties;
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
use strategies::step::utils::StepBacktestingUtils;

#[derive(Debug)]
struct Tick<'a> {
    index: usize,
    value: Option<&'a BasicTickProperties>,
}

#[derive(Debug)]
struct Candle<'a> {
    index: usize,
    value: Option<&'a BasicCandleProperties>,
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

pub struct StepStrategyRunningConfig<'a, P, T, H, U, N, R, D, E>
where
    P: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,

    T: StepBacktestingMainStore,

    H: Helpers,
    U: LevelUtils,
    N: LevelConditions,
    R: OrderUtils,
    D: ChartTracesModifier,
    E: TradingEngine,
{
    pub timeframes: StrategyTimeframes,
    pub stores: &'a mut StepBacktestingStores<T>,
    pub utils: &'a StepBacktestingUtils<H, U, N, R, D, E>,
    pub params: &'a P,
}

pub fn loop_through_historical_data<P, L, T, H, U, N, R, D, E>(
    historical_data: &HistoricalData,
    strategy_config: StepStrategyRunningConfig<P, T, H, U, N, R, D, E>,
    trading_limiter: &L,
    iteration_runner: &impl RunStepBacktestingIteration,
) -> Result<StrategyPerformance>
where
    P: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    L: TradingLimiter,

    T: StepBacktestingMainStore,

    H: Helpers,
    U: LevelUtils,
    N: LevelConditions,
    R: OrderUtils,
    D: ChartTracesModifier,
    E: TradingEngine,
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
            iteration_runner.run_iteration(
                current_tick.clone(),
                if new_candle_appeared {
                    current_candle
                        .value
                        .map(|candle_props| StepBacktestingCandleProperties {
                            base: candle_props.clone(),
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
    use base::entities::candle::CandleVolatility;
    use base::entities::order::{BasicOrderProperties, OrderId, OrderPrice, OrderType};
    use base::entities::tick::{TickPrice, TickTime};
    use base::entities::{Item, Timeframe};
    use base::helpers::{Holiday, NumberOfDaysToExclude};
    use base::notifier::NotificationQueue;
    use base::params::ParamValue;
    use chrono::{NaiveDateTime, Timelike};
    use float_cmp::approx_eq;
    use rust_decimal_macros::dec;
    use strategies::step::utils::backtesting_charts::{
        ChartTraceEntity, ChartTracesModifier, StepBacktestingChartTraces,
    };
    use strategies::step::utils::entities::working_levels::{
        BasicWLProperties, CorridorType, LevelTime, WLId, WLMaxCrossingValue, WLPrice,
    };
    use strategies::step::utils::entities::StatisticsNotifier;
    use strategies::step::utils::helpers::HelpersImpl;
    use strategies::step::utils::level_conditions::MinAmountOfCandles;
    use strategies::step::utils::level_utils::RemoveInvalidWorkingLevelsUtils;
    use strategies::step::utils::order_utils::{
        OrderUtilsImpl, UpdateOrdersBacktestingStores, UpdateOrdersBacktestingUtils,
    };
    use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use strategies::step::utils::stores::{StepBacktestingConfig, StepBacktestingMainStore};

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
    struct TestIterationRunner;

    impl RunStepBacktestingIteration for TestIterationRunner {
        fn run_iteration<T, H, U, N, R, D, E>(
            &self,
            tick: BasicTickProperties,
            candle: Option<StepBacktestingCandleProperties>,
            signals: StrategySignals,
            stores: &mut StepBacktestingStores<T>,
            utils: &StepBacktestingUtils<H, U, N, R, D, E>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        ) -> Result<()>
        where
            T: StepBacktestingMainStore,

            H: Helpers,
            U: LevelUtils,
            N: LevelConditions,
            R: OrderUtils,
            D: ChartTracesModifier,
            E: TradingEngine,
        {
            if signals.cancel_all_orders {
                stores.config.trading_engine.balances.real -= dec!(50.0);
            }

            if !signals.no_trading_mode {
                stores.config.trading_engine.balances.real += dec!(10.0);

                if candle.is_some() {
                    stores.config.trading_engine.balances.real += dec!(20.0);
                }
            }

            Ok(())
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
        fn get_crossed_level<'a, W>(
            &self,
            current_tick_price: TickPrice,
            created_working_levels: &'a [Item<WLId, W>],
        ) -> Option<&'a Item<WLId, W>>
        where
            W: AsRef<BasicWLProperties>,
        {
            todo!()
        }

        fn remove_active_working_levels_with_closed_orders<O>(
            &self,
            _working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
        ) -> Result<()>
        where
            O: Into<StepOrderProperties>,
        {
            unimplemented!()
        }

        fn update_max_crossing_value_of_active_levels<T>(
            &self,
            _working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
            _current_tick_price: TickPrice,
        ) -> Result<()>
        where
            T: Into<BasicWLProperties>,
        {
            unimplemented!()
        }

        fn remove_invalid_working_levels<W, C, E, T, N, O>(
            &self,
            current_tick: &BasicTickProperties,
            current_volatility: CandleVolatility,
            utils: RemoveInvalidWorkingLevelsUtils<W, C, E, T, O>,
            params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
            entity: StatisticsNotifier<N>,
        ) -> Result<()>
        where
            T: Into<BasicWLProperties>,
            O: AsRef<BasicOrderProperties>,
            W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
            C: LevelConditions,
            E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
            N: NotificationQueue,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestLevelConditionsImpl;

    impl LevelConditions for TestLevelConditionsImpl {
        fn level_exceeds_amount_of_candles_in_corridor(
            &self,
            _level_id: &str,
            _working_level_store: &impl StepWorkingLevelStore,
            _corridor_type: CorridorType,
            _min_amount_of_candles: MinAmountOfCandles,
        ) -> Result<bool> {
            unimplemented!()
        }

        fn price_is_beyond_stop_loss(
            &self,
            _current_tick_price: TickPrice,
            _stop_loss_price: OrderPrice,
            _working_level_type: OrderType,
        ) -> bool {
            unimplemented!()
        }

        fn level_expired_by_distance(
            &self,
            level_price: WLPrice,
            current_tick_price: TickPrice,
            distance_from_level_for_its_deletion: ParamValue,
        ) -> bool {
            unimplemented!()
        }

        fn level_expired_by_time(
            &self,
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
            &self,
            level: &impl AsRef<BasicWLProperties>,
            max_crossing_value: Option<WLMaxCrossingValue>,
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
            current_tick_price: TickPrice,
        ) -> bool {
            unimplemented!()
        }

        fn level_has_no_active_orders<T>(&self, level_orders: &[T]) -> bool
        where
            T: AsRef<BasicOrderProperties>,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestOrderUtilsImpl;

    impl OrderUtils for TestOrderUtilsImpl {
        fn get_new_chain_of_orders<W>(
            &self,
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

        fn update_orders_backtesting<M, T, C, L>(
            &self,
            _current_tick: &BasicTickProperties,
            _current_candle: &StepBacktestingCandleProperties,
            _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
            _stores: UpdateOrdersBacktestingStores<M>,
            _utils: UpdateOrdersBacktestingUtils<T, C, L>,
            _no_trading_mode: bool,
        ) -> Result<()>
        where
            M: BasicOrderStore<OrderProperties = StepOrderProperties>
                + StepWorkingLevelStore<WorkingLevelProperties = BacktestingWLProperties>,
            T: TradingEngine,
            C: ChartTracesModifier,
            L: LevelConditions,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestChartTracesModifierImpl;

    impl ChartTracesModifier for TestChartTracesModifierImpl {
        fn add_entity_to_chart_traces(
            &self,
            _entity: ChartTraceEntity,
            _chart_traces: &mut StepBacktestingChartTraces,
            _current_candle: &StepBacktestingCandleProperties,
        ) {
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
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 18:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                None,
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 20:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 21:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 22:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 23:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 00:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 01:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                None,
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 03:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 04:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("18-05-2022 05:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
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

        let utils = StepBacktestingUtils {
            helpers: TestHelpersImpl::default(),
            level_utils: TestLevelUtilsImpl::default(),
            level_conditions: TestLevelConditionsImpl::default(),
            order_utils: TestOrderUtilsImpl::default(),
            chart_traces_modifier: TestChartTracesModifierImpl::default(),
            trading_engine: TestTradingEngineImpl::default(),
        };

        let strategy_config = StepStrategyRunningConfig {
            timeframes: StrategyTimeframes {
                candle: Timeframe::Hour,
                tick: Timeframe::ThirtyMin,
            },
            stores: &mut step_stores,
            utils: &utils,
            params: &step_params,
        };

        let iteration_runner = TestIterationRunner::default();

        let strategy_performance = loop_through_historical_data(
            &historical_data,
            strategy_config,
            &trading_limiter,
            &iteration_runner,
        )
        .unwrap();

        assert_eq!(strategy_performance, dec!(2.6));
        assert_eq!(
            step_stores.config.trading_engine.balances.real,
            dec!(10_260)
        );
    }
}
