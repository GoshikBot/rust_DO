use anyhow::{Context, Result};
use base::entities::candle::BasicCandle;
use base::entities::{BasicTick, StrategyProperties};
use base::historical_data::synchronization::{sync_candles_and_ticks, HistoricalData};
use base::requests::ureq::Ureq;
use chrono::{DateTime, Duration, Utc};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use std::cmp::Ordering;
use base::historical_data::serialization;
use trading_apis::metaapi_market_data_api::{LoggerTarget, Timeframe};
use trading_apis::{MarketDataApi, MetaapiMarketDataApi};

fn main() -> Result<()> {
    let _ = backtest_step_strategy(
        StrategyProperties {
            symbol: String::from("GBPUSDm"),
            candle_timeframe: Timeframe::Hour,
            tick_timeframe: Timeframe::OneMin,
            end_time: DateTime::from(
                DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
            ),
            duration: Duration::weeks(16),
        },
        None,
    )?;

    Ok(())
}

fn backtest_step_strategy(
    properties: StrategyProperties,
    logger_target: Option<LoggerTarget>,
) -> Result<()> {
    dotenv::dotenv().unwrap();

    let StrategyProperties {
        symbol,
        candle_timeframe,
        tick_timeframe,
        end_time,
        duration,
    } = properties;

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let metaapi: MetaapiMarketDataApi<Ureq> =
        MetaapiMarketDataApi::new(auth_token, account_id, logger_target, Default::default());


    let historical_data = serialization::try_to_deserialize_historical_data(&properties)

    let candles = metaapi.get_historical_candles(&symbol, candle_timeframe, end_time, duration)?;
    let ticks = metaapi.get_historical_ticks(&symbol, tick_timeframe, end_time, duration)?;

    let HistoricalData { candles, ticks } =
        sync_candles_and_ticks(HistoricalData { candles, ticks })
            .context("error on synchronizing ticks and candles")?;
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
