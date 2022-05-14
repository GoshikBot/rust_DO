use chrono::{DateTime, Duration};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use trading_apis::entities::HistoricalTimeframe;
use trading_apis::{MarketDataApi, MetaapiMarketDataApi};

fn main() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let timeframe = HistoricalTimeframe::Hour;

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        String::from("test"),
        Default::default(),
    );

    let end_time = DateTime::from(
        DateTime::parse_from_str("2022-03-01 01:00 +0000", "%Y-%m-%d %H:%M %z").unwrap(),
    );

    let duration = Duration::weeks(12);

    let candles = metaapi
        .get_historical_candles(symbol, timeframe, end_time, duration)
        .unwrap();

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
