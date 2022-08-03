use anyhow::{Context, Result};
use backtesting::historical_data::serialization::{
    HistoricalDataCsvSerialization, HistoricalDataSerialization,
};
use backtesting::historical_data::synchronization::sync_candles_and_ticks;
use backtesting::historical_data::{get_historical_data, serialization, synchronization};
use backtesting::StrategyInitConfig;
use base::entities::candle::BasicCandleProperties;
use base::entities::{BasicTickProperties, StrategyTimeframes, Timeframe};
use base::requests::ureq::UreqRequestApi;
use chrono::{DateTime, Duration, Utc};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use strategies::step::utils::trading_limiter;
use trading_apis::metaapi_market_data_api::{ApiData, ApiUrls, TargetLogger};
use trading_apis::MetaapiMarketDataApi;

use base::params::{StrategyCsvFileParams, StrategyParams};
use strategies::step::step_backtesting;
use strategies::step::utils::entities::params::{StepPointParam, StepRatioParam};
use strategies::step::utils::entities::{StrategyPerformance, StrategySignals};
use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use strategies::step::utils::stores::in_memory_step_realtime_config_store::InMemoryStepRealtimeConfigStore;
use strategies::step::utils::stores::step_realtime_config_store::StepRealtimeConfigStore;
use strategies::step::utils::stores::{
    StepBacktestingBalances, StepBacktestingConfig, StepBacktestingStores,
};
use strategies::step::utils::trading_limiter::TradingLimiterBacktesting;
use strategy_runners::step::backtesting_runner;
use strategy_runners::step::backtesting_runner::StepStrategyRunningConfig;

const AUTH_TOKEN: &str = "AUTH_TOKEN";
const DEMO_ACCOUNT_ID: &str = "DEMO_ACCOUNT_ID";

const STEP_HISTORICAL_DATA_FOLDER_ENV: &str = "STEP_HISTORICAL_DATA_FOLDER";
const STEP_PARAMS_CSV_FILE: &str = "STEP_PARAMS_CSV_FILE";

fn main() -> Result<()> {
    dotenv::from_filename("common.env").unwrap();
    dotenv::from_filename("step.env").unwrap();
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    backtest_step_strategy(StrategyInitConfig {
        symbol: String::from("GBPUSDm"),
        timeframes: StrategyTimeframes {
            candle: Timeframe::Hour,
            tick: Timeframe::OneMin,
        },
        end_time: DateTime::from(
            DateTime::parse_from_str("27-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
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
        auth_token: dotenv::var("AUTH_TOKEN").unwrap(),
        account_id: dotenv::var("DEMO_ACCOUNT_ID").unwrap(),
        urls: ApiUrls {
            main: dotenv::var("MAIN_API_URL").unwrap(),
            market_data: dotenv::var("MARKET_DATA_API_URL").unwrap()
        }
    };

    let request_api = UreqRequestApi::new();

    let market_data_api =
        MetaapiMarketDataApi::new(api_data, Default::default(), request_api);

    let step_historical_data_folder = dotenv::var(STEP_HISTORICAL_DATA_FOLDER_ENV).unwrap();

    let historical_data_csv_serialization = HistoricalDataCsvSerialization::new();

    let historical_data = get_historical_data(
        step_historical_data_folder,
        &strategy_properties,
        &market_data_api,
        &historical_data_csv_serialization,
        sync_candles_and_ticks,
    )?;

    let mut step_stores: StepBacktestingStores = Default::default();

    let step_params_csv_file = dotenv::var(STEP_PARAMS_CSV_FILE).unwrap();
    let step_params: StrategyCsvFileParams<StepPointParam, StepRatioParam> =
        StrategyCsvFileParams::new(step_params_csv_file)?;

    let trading_limiter = TradingLimiterBacktesting::new();

    let strategy_performance = backtesting_runner::loop_through_historical_data(
        &historical_data,
        StepStrategyRunningConfig {
            timeframes: strategy_properties.timeframes,
            stores: &mut step_stores,
            params: &step_params,
        },
        &trading_limiter,
        step_backtesting::run_iteration,
    )?;

    println!("Strategy performance: {}", strategy_performance);

    Ok(())
}

fn plot_results(candles: Vec<BasicCandleProperties>) {
    let x = candles
        .iter()
        .map(|candle| candle.main.time.format("%Y-%m-%d %H:%M:%S").to_string())
        .collect::<Vec<_>>();

    let open = candles
        .iter()
        .map(|candle| candle.edge_prices.open)
        .collect::<Vec<_>>();
    let high = candles
        .iter()
        .map(|candle| candle.edge_prices.high)
        .collect::<Vec<_>>();
    let low = candles
        .iter()
        .map(|candle| candle.edge_prices.low)
        .collect::<Vec<_>>();
    let close = candles
        .iter()
        .map(|candle| candle.edge_prices.close)
        .collect::<Vec<_>>();

    let trace1 = Candlestick::new(x, open, high, low, close);

    let layout = Layout::new().y_axis(Axis::new().fixed_range(false));

    let mut plot = Plot::new();
    plot.add_trace(trace1);

    plot.set_layout(layout);

    plot.show();

    plot.to_inline_html(Some("simple_candlestick_chart"));
}
