use anyhow::{Context, Result};
use base::entities::candle::BasicCandle;
use base::entities::BasicTick;
use base::requests::ureq::Ureq;
use chrono::{DateTime, Duration, Utc};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use std::cmp::Ordering;
use trading_apis::metaapi_market_data_api::{LoggerTarget, Timeframe};
use trading_apis::{MarketDataApi, MetaapiMarketDataApi};

fn main() -> Result<()> {
    let _ = backtest_step_strategy(
        "GBPUSDm",
        Timeframe::Hour,
        Timeframe::OneMin,
        DateTime::from(
            DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
        ),
        Duration::weeks(16),
        None,
    )?;

    Ok(())
}

fn backtest_step_strategy(
    symbol: &str,
    candle_timeframe: Timeframe,
    tick_timeframe: Timeframe,
    end_time: DateTime<Utc>,
    duration: Duration,
    logger_target: Option<LoggerTarget>,
) -> Result<()> {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let metaapi: MetaapiMarketDataApi<Ureq> =
        MetaapiMarketDataApi::new(auth_token, account_id, logger_target, Default::default());

    let candles = metaapi.get_historical_candles(symbol, candle_timeframe, end_time, duration)?;

    let ticks = metaapi.get_historical_ticks(symbol, tick_timeframe, end_time, duration)?;

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
