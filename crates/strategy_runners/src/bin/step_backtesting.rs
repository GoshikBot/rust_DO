use anyhow::{Context, Result};
use backtesting::historical_data::serialization::{
    HistoricalDataCsvSerialization, HistoricalDataSerialization,
};
use backtesting::historical_data::synchronization::sync_candles_and_ticks;
use backtesting::historical_data::{get_historical_data, serialization, synchronization};
use base::entities::candle::BasicCandle;
use base::entities::{BasicTick, HistoricalData, StrategyProperties, Timeframe};
use base::requests::ureq::UreqRequestApi;
use chrono::{DateTime, Duration, Utc};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use trading_apis::metaapi_market_data_api::LoggerTarget;
use trading_apis::MetaapiMarketDataApi;

const AUTH_TOKEN: &str = "AUTH_TOKEN";
const DEMO_ACCOUNT_ID: &str = "DEMO_ACCOUNT_ID";
const STEP_HISTORICAL_DATA_FOLDER_ENV: &str = "STEP_HISTORICAL_DATA_FOLDER";

fn main() -> Result<()> {
    backtest_step_strategy(
        StrategyProperties {
            symbol: String::from("GBPUSDm"),
            candle_timeframe: Timeframe::Hour,
            tick_timeframe: Timeframe::OneMin,
            end_time: DateTime::from(
                DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
            ),
            duration: Duration::weeks(16),
        },
        "",
    )?;

    Ok(())
}

fn backtest_step_strategy(
    strategy_properties: StrategyProperties,
    logger_target: &str,
) -> Result<()> {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var(AUTH_TOKEN).unwrap();
    let account_id = dotenv::var(DEMO_ACCOUNT_ID).unwrap();

    let request_api = UreqRequestApi::new();

    let market_data_api = MetaapiMarketDataApi::new(
        &auth_token,
        &account_id,
        logger_target,
        Default::default(),
        &request_api,
    );

    let step_historical_data_folder = dotenv::var(STEP_HISTORICAL_DATA_FOLDER_ENV).unwrap();

    let historical_data_csv_serialization = HistoricalDataCsvSerialization::new();

    let historical_data = get_historical_data(
        step_historical_data_folder,
        &strategy_properties,
        &market_data_api,
        &historical_data_csv_serialization,
        sync_candles_and_ticks,
    )?;

    Ok(())
}

fn plot_results(candles: Vec<BasicCandle>) {
    let x = candles
        .iter()
        .map(|candle| {
            candle
                .properties
                .time
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
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
