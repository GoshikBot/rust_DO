use anyhow::Result;
use backtesting::historical_data::get_historical_data;
use backtesting::historical_data::serialization::HistoricalDataCsvSerialization;
use backtesting::historical_data::synchronization::sync_candles_and_ticks;
use backtesting::trading_engine::BacktestingTradingEngine;
use backtesting::{HistoricalData, StrategyInitConfig};
use base::corridor::BasicCorridorUtilsImpl;
use base::entities::candle::BasicCandleProperties;
use base::entities::{StrategyTimeframes, Timeframe, CANDLE_TIMEFRAME_ENV, TICK_TIMEFRAME_ENV};
use base::helpers::exclude_weekend_and_holidays;
use base::requests::ureq::UreqRequestApi;
use chrono::{DateTime, Duration};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use std::str::FromStr;
use trading_apis::metaapi_market_data_api::{ApiData, ApiUrls};
use trading_apis::MetaapiMarketDataApi;

use base::params::StrategyCsvFileParams;
use strategies::step::step_backtesting::run_iteration;
use strategies::step::utils::backtesting_charts::add_entity_to_chart_traces;
use strategies::step::utils::corridors::CorridorsImpl;
use strategies::step::utils::entities::candle::StepCandleProperties;
use strategies::step::utils::entities::params::{StepPointParam, StepRatioParam};
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

const AUTH_TOKEN_ENV: &str = "AUTH_TOKEN";
const DEMO_ACCOUNT_ID_ENV: &str = "DEMO_ACCOUNT_ID";
const MAIN_API_URL_ENV: &str = "MAIN_API_URL";
const MARKET_DATA_API_URL_ENV: &str = "MARKET_DATA_API_URL";

const STEP_HISTORICAL_DATA_FOLDER_ENV: &str = "STEP_HISTORICAL_DATA_FOLDER";
const STEP_PARAMS_CSV_FILE_ENV: &str = "STEP_PARAMS_CSV_FILE";

fn main() -> Result<()> {
    dotenv::from_filename("common.env").unwrap();
    dotenv::from_filename("step.env").unwrap();

    let candle_timeframe =
        Timeframe::from_str(&dotenv::var(CANDLE_TIMEFRAME_ENV).unwrap()).unwrap();
    let tick_timeframe = Timeframe::from_str(&dotenv::var(TICK_TIMEFRAME_ENV).unwrap()).unwrap();

    backtest_step_strategy(StrategyInitConfig {
        symbol: String::from("GBPUSDm"),
        timeframes: StrategyTimeframes {
            candle: candle_timeframe,
            tick: tick_timeframe,
        },
        end_time: DateTime::from(
            DateTime::parse_from_str("01-08-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
        ),
        duration: Duration::weeks(11),
    })?;

    Ok(())
}

fn backtest_step_strategy(strategy_properties: StrategyInitConfig) -> Result<()> {
    if (strategy_properties.timeframes.candle as u32) < (strategy_properties.timeframes.tick as u32)
    {
        anyhow::bail!("candle timeframe should be bigger than tick timeframe");
    }

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
        &strategy_properties,
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

    let mut step_stores = StepBacktestingStores {
        main: InMemoryStepBacktestingStore::new(),
        config: StepBacktestingConfig::default(historical_data.candles.len()),
        statistics: Default::default(),
    };

    let step_params_csv_file = dotenv::var(STEP_PARAMS_CSV_FILE_ENV).unwrap();
    let step_params: StrategyCsvFileParams<StepPointParam, StepRatioParam> =
        StrategyCsvFileParams::new(step_params_csv_file)?;

    let trading_limiter = TradingLimiterBacktesting::new();

    let utils: StepBacktestingUtils<
        HelpersImpl,
        LevelUtilsImpl,
        LevelConditionsImpl,
        OrderUtilsImpl,
        BasicCorridorUtilsImpl,
        CorridorsImpl,
        _,
        _,
        _,
    > = StepBacktestingUtils::new(
        add_entity_to_chart_traces,
        BacktestingTradingEngine::new(),
        exclude_weekend_and_holidays,
    );

    let strategy_performance = backtesting_runner::loop_through_historical_data(
        &historical_data,
        StepStrategyRunningConfig {
            timeframes: strategy_properties.timeframes,
            stores: &mut step_stores,
            utils: &utils,
            params: &step_params,
        },
        &trading_limiter,
        &get_candle_leading_price,
        &run_iteration,
    )?;

    println!("Strategy performance: {}", strategy_performance);

    Ok(())
}

fn _plot_results(candles: Vec<Option<BasicCandleProperties>>) {
    let x = candles
        .iter()
        .map(|candle| {
            candle
                .clone()
                .map(|c| c.time.format("%Y-%m-%d %H:%M:%S").to_string())
        })
        .collect::<Vec<_>>();

    let open = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.prices.open))
        .collect::<Vec<_>>();
    let high = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.prices.high))
        .collect::<Vec<_>>();
    let low = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.prices.low))
        .collect::<Vec<_>>();
    let close = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.prices.close))
        .collect::<Vec<_>>();

    let trace1 = Candlestick::new(x, open, high, low, close);

    let layout = Layout::new().y_axis(Axis::new().fixed_range(false));

    let mut plot = Plot::new();
    plot.add_trace(trace1);

    plot.set_layout(layout);

    plot.to_html("/home/nikmas/candlestick_chart.html");
}
