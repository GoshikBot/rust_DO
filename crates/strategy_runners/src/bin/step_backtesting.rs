use base::entities::candle::BasicCandle;
use chrono::{DateTime, Duration};
use plotly::layout::Axis;
use plotly::{Candlestick, Layout, Plot};
use trading_apis::metaapi_market_data_api::Timeframe;
use trading_apis::{MarketDataApi, MetaapiMarketDataApi};

fn main() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let candle_timeframe = Timeframe::Hour;
    let tick_timeframe = Timeframe::OneMin;

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        String::from("test"),
        Default::default(),
    );

    let end_time = DateTime::from(
        DateTime::parse_from_str("14-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
    );

    let duration = Duration::weeks(2);

    let candles = metaapi
        .get_historical_candles(symbol, candle_timeframe, end_time, duration)
        .unwrap();

    let ticks = metaapi
        .get_historical_candles(symbol, tick_timeframe, end_time, duration)
        .unwrap();

    // println!("first tick {}", ticks.first().unwrap().properties.time);
    // println!("last tick {}", ticks.last().unwrap().properties.time);
    // println!("first candle {}", candles.first().unwrap().properties.time);
    // println!("last candle {}", candles.last().unwrap().properties.time);
    //
    // plot_results(candles);
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
