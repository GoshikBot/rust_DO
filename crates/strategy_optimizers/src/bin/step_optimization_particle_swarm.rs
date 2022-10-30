use anyhow::Result;
use argmin::core::observers::{ObserverMode, SlogLogger};
use argmin::core::{CostFunction, Executor, IterState, OptimizationResult, PopulationState};
use argmin::solver::particleswarm::{Particle, ParticleSwarm};
use argmin::solver::simulatedannealing::{Anneal, SATempFunc, SimulatedAnnealing};
use backtesting::historical_data::get_historical_data;
use backtesting::historical_data::serialization::HistoricalDataCsvSerialization;
use backtesting::historical_data::synchronization::sync_candles_and_ticks;
use backtesting::trading_engine::BacktestingTradingEngine;
use backtesting::{HistoricalData, StrategyInitConfig};
use base::corridor::BasicCorridorUtilsImpl;
use base::entities::tick::HistoricalTickPrice;
use base::entities::{
    BasicTickProperties, StrategyTimeframes, Timeframe, CANDLE_TIMEFRAME_ENV, TICK_TIMEFRAME_ENV,
};
use base::helpers::exclude_weekend_and_holidays;
use base::params::{StrategyMultiSourcingParams, StrategyParam, StrategyParams};
use base::requests::ureq::UreqRequestApi;
use chrono::{DateTime, Duration};
use rand::distributions::Uniform;
use rand::prelude::*;
use rand_xoshiro::Xoshiro256PlusPlus;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::env;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use strategies::step::step_backtesting::run_iteration;
use strategies::step::utils::angle_utils::AngleUtilsImpl;
use strategies::step::utils::backtesting_charts::add_entity_to_chart_traces;
use strategies::step::utils::corridors::CorridorsImpl;
use strategies::step::utils::entities::candle::StepCandleProperties;
use strategies::step::utils::entities::params::{StepPointParam, StepRatioParam};
use strategies::step::utils::entities::{
    StrategyPerformance, MODE_ENV, STEP_HISTORICAL_DATA_FOLDER_ENV,
};
use strategies::step::utils::helpers::HelpersImpl;
use strategies::step::utils::level_conditions::LevelConditionsImpl;
use strategies::step::utils::level_utils::LevelUtilsImpl;
use strategies::step::utils::order_utils::OrderUtilsImpl;
use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use strategies::step::utils::stores::{StepBacktestingConfig, StepBacktestingStores};
use strategies::step::utils::trading_limiter::TradingLimiterBacktesting;
use strategies::step::utils::{get_candle_leading_price, StepBacktestingUtils};
use strategy_runners::step::backtesting_runner;
use strategy_runners::step::backtesting_runner::StepStrategyRunningConfig;
use trading_apis::metaapi_market_data_api::{
    ApiData, ApiUrls, AUTH_TOKEN_ENV, DEMO_ACCOUNT_ID_ENV, MAIN_API_URL_ENV,
    MARKET_DATA_API_URL_ENV,
};

use trading_apis::MetaapiMarketDataApi;

type OptimizationParamValue = f64;
type OptimizationParamBounds = (OptimizationParamValue, OptimizationParamValue);

type LowerUpperParamBounds = (Vec<OptimizationParamValue>, Vec<OptimizationParamValue>);

const NUMBER_OF_PARTICLES: usize = 500;
const MAX_NUMBER_OF_ITERATIONS: u64 = 100;

#[derive(Debug, Clone, Copy)]
enum OptimizationParamDescr {
    Point {
        name: StepPointParam,
        num_type: NumType,
    },
    Ratio(StepRatioParam),
}

#[derive(Debug, Copy, Clone)]
enum NumType {
    Integer,
    Float,
}

#[derive(Debug, Clone)]
struct OptimizationInitialParam {
    descr: OptimizationParamDescr,
    bounds: OptimizationParamBounds,
}

type OptimizationPerformance = f64;

type StepOptimizationResult = OptimizationResult<
    StepStrategyOptimization,
    ParticleSwarm<Vec<OptimizationParamValue>, OptimizationPerformance>,
    PopulationState<
        Particle<Vec<OptimizationParamValue>, OptimizationPerformance>,
        OptimizationPerformance,
    >,
>;

struct StepStrategyOptimization {
    param_descrs: Vec<OptimizationParamDescr>,
    historical_data: HistoricalData<StepCandleProperties, BasicTickProperties<HistoricalTickPrice>>,
    strategy_config: StrategyInitConfig,
}

impl StepStrategyOptimization {
    pub fn new(
        params: Vec<OptimizationInitialParam>,
        historical_data: HistoricalData<
            StepCandleProperties,
            BasicTickProperties<HistoricalTickPrice>,
        >,
        strategy_config: StrategyInitConfig,
    ) -> (Self, LowerUpperParamBounds) {
        let lower_bound = params
            .iter()
            .map(|param| param.bounds.0)
            .collect::<Vec<_>>();

        let upper_bound = params
            .iter()
            .map(|param| param.bounds.1)
            .collect::<Vec<_>>();

        let param_descrs = params
            .into_iter()
            .map(|param| param.descr)
            .collect::<Vec<_>>();

        (
            StepStrategyOptimization {
                param_descrs,
                historical_data,
                strategy_config,
            },
            (lower_bound, upper_bound),
        )
    }

    fn to_strategy_params(
        &self,
        params: &[OptimizationParamValue],
    ) -> Result<StrategyMultiSourcingParams<StepPointParam, StepRatioParam>> {
        let mut strategy_params = Vec::new();
        for param in params.iter().zip(self.param_descrs.iter()) {
            strategy_params.push(match param.1 {
                OptimizationParamDescr::Point { name, num_type } => StrategyParam {
                    name: name.to_string(),
                    value: match num_type {
                        NumType::Integer => param.0.trunc().to_string(),
                        NumType::Float => param.0.to_string(),
                    },
                },
                OptimizationParamDescr::Ratio(name) => StrategyParam {
                    name: name.to_string(),
                    value: format!("{}k", param.0),
                },
            });
        }

        StrategyMultiSourcingParams::from_vec(strategy_params)
    }
}

impl CostFunction for StepStrategyOptimization {
    type Param = Vec<OptimizationParamValue>;
    type Output = OptimizationPerformance;

    fn cost(&self, param: &Self::Param) -> Result<Self::Output> {
        let step_params = self.to_strategy_params(param)?;

        if step_params.get_ratio_param_value(StepRatioParam::DistanceToMoveTakeProfits, 1)
            > step_params.get_ratio_param_value(StepRatioParam::DistanceFromLevelToFirstOrder, 1)
        {
            return Ok(Self::Output::MAX);
        }

        println!("----------------------------------------------");
        println!("{}", step_params);

        let mut step_stores = StepBacktestingStores {
            main: InMemoryStepBacktestingStore::new(),
            config: StepBacktestingConfig::default(self.historical_data.candles.len()),
            statistics: Default::default(),
        };

        let utils: StepBacktestingUtils<
            HelpersImpl,
            LevelUtilsImpl,
            LevelConditionsImpl,
            OrderUtilsImpl,
            BasicCorridorUtilsImpl,
            CorridorsImpl,
            AngleUtilsImpl,
            _,
            _,
            _,
        > = StepBacktestingUtils::new(
            add_entity_to_chart_traces,
            exclude_weekend_and_holidays,
            BacktestingTradingEngine::new(),
        );

        let trading_limiter = TradingLimiterBacktesting::new();

        let performance = backtesting_runner::loop_through_historical_data(
            &self.historical_data,
            StepStrategyRunningConfig {
                timeframes: self.strategy_config.timeframes,
                stores: &mut step_stores,
                utils: &utils,
                params: &step_params,
            },
            &trading_limiter,
            &run_iteration,
        )
        .unwrap_or(Decimal::MIN);

        println!("Performance: {}", performance);
        println!("----------------------------------------------");

        Ok((performance * dec!(-1))
            .to_string()
            .parse::<Self::Output>()?)
    }
}

fn optimize_step(
    params: Vec<OptimizationInitialParam>,
    historical_data: HistoricalData<StepCandleProperties, BasicTickProperties<HistoricalTickPrice>>,
    strategy_config: StrategyInitConfig,
) -> Result<StepOptimizationResult> {
    let (cost_function, bounds) =
        StepStrategyOptimization::new(params, historical_data, strategy_config);

    let solver = ParticleSwarm::new(bounds, NUMBER_OF_PARTICLES);

    let result = Executor::new(cost_function, solver)
        .configure(|state| state.max_iters(MAX_NUMBER_OF_ITERATIONS))
        .add_observer(SlogLogger::term(), ObserverMode::Always)
        .run()?;

    // Wait a second (lets the logger flush everything before printing again)
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(result)
}

fn main() -> Result<()> {
    dotenv::from_filename("common.env").unwrap();
    dotenv::from_filename("step.env").unwrap();

    let candle_timeframe = "1h";
    env::set_var(CANDLE_TIMEFRAME_ENV, candle_timeframe);
    let candle_timeframe = Timeframe::from_str(candle_timeframe).unwrap();

    let tick_timeframe = "5m";
    env::set_var(TICK_TIMEFRAME_ENV, tick_timeframe);
    let tick_timeframe = Timeframe::from_str(tick_timeframe).unwrap();

    env::set_var(MODE_ENV, "optimization");

    let strategy_config = StrategyInitConfig {
        symbol: String::from("GBPUSDm"),
        timeframes: StrategyTimeframes {
            candle: candle_timeframe,
            tick: tick_timeframe,
        },
        end_time: DateTime::from(
            DateTime::parse_from_str("10-06-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
        ),
        duration: Duration::weeks(36),
    };

    let params = vec![
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name: StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct,
                num_type: NumType::Float,
            },
            bounds: (20., 50.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name: StepPointParam::AmountOfOrders,
                num_type: NumType::Integer,
            },
            bounds: (5.5, 6.5),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name: StepPointParam::LevelExpirationDays,
                num_type: NumType::Integer,
            },
            bounds: (3., 40.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name:
                StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel,
                num_type: NumType::Integer,
            },
            bounds: (3., 5.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name:
                StepPointParam::MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel,
                num_type: NumType::Integer,
            },
            bounds: (12., 30.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name: StepPointParam::MinAmountOfCandlesInCorridorDefiningEdgeBargaining,
                num_type: NumType::Integer,
            },
            bounds: (4., 10.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Point {
                name: StepPointParam::MaxLossPerOneChainOfOrdersPctOfBalance,
                num_type: NumType::Integer,
            },
            bounds: (15., 15.), // fix single value
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles,
            ),
            bounds: (0.6, 3.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::MinDistanceBetweenCurrentMaxAndMinAnglesForNewInnerAngleToAppear,
            ),
            bounds: (2., 3.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(StepRatioParam::MinBreakDistance),
            bounds: (0.1, 0.6),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(StepRatioParam::DistanceFromLevelToFirstOrder),
            bounds: (0.5, 2.2),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(StepRatioParam::DistanceFromLevelToStopLoss),
            bounds: (2.3, 6.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::DistanceFromLevelForSignalingOfMovingTakeProfits,
            ),
            bounds: (0.1, 4.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(StepRatioParam::DistanceToMoveTakeProfits),
            bounds: (0.1, 0.5),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(StepRatioParam::DistanceFromLevelForItsDeletion),
            bounds: (9.22, 90.),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel,
            ),
            bounds: (0.1, 0.6),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::DistanceDefiningNearbyLevelsOfTheSameType,
            ),
            bounds: (0.5, 2.3),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion,
            ),
            bounds: (0.5, 1.18),
        },
        OptimizationInitialParam {
            descr: OptimizationParamDescr::Ratio(
                StepRatioParam::RangeOfBigCorridorNearLevel,
            ),
            bounds: (1.2, 3.),
        },
    ];

    let api_data = ApiData {
        auth_token: dotenv::var(AUTH_TOKEN_ENV).unwrap(),
        account_id: dotenv::var(DEMO_ACCOUNT_ID_ENV).unwrap(),
        urls: ApiUrls {
            main: dotenv::var(MAIN_API_URL_ENV).unwrap(),
            market_data: dotenv::var(MARKET_DATA_API_URL_ENV).unwrap(),
        },
    };

    let request_api = UreqRequestApi::new();

    let market_data_api = MetaapiMarketDataApi::new(api_data, Default::default(), request_api);

    let step_historical_data_folder = dotenv::var(STEP_HISTORICAL_DATA_FOLDER_ENV).unwrap();

    let historical_data_csv_serialization = HistoricalDataCsvSerialization::new();

    let historical_data = get_historical_data(
        step_historical_data_folder,
        &strategy_config,
        &market_data_api,
        &historical_data_csv_serialization,
        sync_candles_and_ticks,
    )?;

    let historical_data = HistoricalData {
        candles: historical_data
            .candles
            .into_iter()
            .map(|candle| {
                candle.map(|c| {
                    let leading_price = get_candle_leading_price(&c);

                    StepCandleProperties {
                        base: c,
                        leading_price,
                    }
                })
            })
            .collect(),
        ticks: historical_data.ticks,
    };

    let now = Instant::now();
    let result = optimize_step(params, historical_data, strategy_config)?;
    println!("Optimization took {} minutes", now.elapsed().as_secs() / 60);

    println!("Optimization result: {}", result);

    Ok(())
}
