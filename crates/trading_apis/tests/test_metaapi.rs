use base::entities::Timeframe;
use base::requests::ureq::UreqRequestApi;
use chrono::{DateTime, Duration, Utc};
use std::time::Instant;
use trading_apis::metaapi_market_data_api::DAYS_FOR_VOLATILITY;
use trading_apis::{MarketDataApi, MetaapiMarketDataApi, RetrySettings};

#[test]
#[ignore]
fn should_successfully_get_current_tick() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let request_api = UreqRequestApi::new();

    let metaapi =
        MetaapiMarketDataApi::new(auth_token, account_id, "", Default::default(), request_api);

    metaapi.get_current_tick(symbol).unwrap();
}

#[test]
#[ignore]
fn should_return_an_error_after_defined_retries_of_getting_current_tick() {
    dotenv::dotenv().unwrap();

    let auth_token = String::from("invalid");
    let account_id = String::from("invalid");

    let symbol = "GBPUSDm";
    let number_of_request_retries = 3;
    let seconds_to_sleep_before_request_retry = 1;

    let request_api = UreqRequestApi::new();

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        "",
        RetrySettings {
            number_of_request_retries,
            seconds_to_sleep_before_request_retry,
        },
        request_api,
    );

    // check that the method takes at least min amount of time to execute
    // to make sure the retries of a request work
    let start = Instant::now();
    assert!(metaapi.get_current_tick(symbol).is_err());
    let duration = start.elapsed().as_secs();

    let min_amount_of_time_for_method_execution =
        seconds_to_sleep_before_request_retry * number_of_request_retries;
    assert!(duration >= min_amount_of_time_for_method_execution as u64);
}

#[test]
#[ignore]
fn should_successfully_get_current_candle() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let timeframe = Timeframe::Hour;

    let request_api = UreqRequestApi::new();

    let metaapi =
        MetaapiMarketDataApi::new(auth_token, account_id, "", Default::default(), request_api);

    assert!(metaapi.get_current_candle(symbol, timeframe).is_ok());
}

#[test]
#[ignore]
fn should_return_an_error_after_defined_retries_of_getting_current_candle() {
    dotenv::dotenv().unwrap();

    let auth_token = String::from("invalid");
    let account_id = String::from("invalid");

    let symbol = "GBPUSDm";
    let timeframe = Timeframe::Hour;
    let number_of_request_retries = 3;
    let seconds_to_sleep_before_request_retry = 1;

    let request_api = UreqRequestApi::new();

    let metaapi: MetaapiMarketDataApi<UreqRequestApi> = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        "",
        RetrySettings {
            number_of_request_retries,
            seconds_to_sleep_before_request_retry,
        },
        request_api,
    );

    // check that the method takes at least min amount of time to execute
    // to make sure the retries of a request work
    let start = Instant::now();
    assert!(metaapi.get_current_candle(symbol, timeframe).is_err());
    let duration = start.elapsed().as_secs();

    let min_amount_of_time_for_method_execution =
        seconds_to_sleep_before_request_retry * number_of_request_retries;
    assert!(duration >= min_amount_of_time_for_method_execution as u64);
}

#[test]
#[ignore]
fn should_successfully_get_hourly_historical_candles() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let timeframe = Timeframe::Hour;

    let request_api = UreqRequestApi::new();

    let metaapi: MetaapiMarketDataApi<UreqRequestApi> =
        MetaapiMarketDataApi::new(auth_token, account_id, "", Default::default(), request_api);

    let end_time = Utc::now();

    let duration = Duration::weeks(12);

    let candles = metaapi.get_historical_candles(symbol, timeframe, end_time, duration);

    assert!(candles.is_ok());

    let candles = candles.unwrap();

    assert_eq!(
        candles.iter().filter(|candle| candle.is_some()).count() as i64,
        duration.num_hours() - Duration::days(DAYS_FOR_VOLATILITY as i64).num_hours() + 1
    );

    // test for proper amount of nones between adjacent candles
    let mut number_of_nones_in_row = 0;
    let mut previous_candle = candles.first().unwrap().as_ref().unwrap();
    for candle in &candles[1..] {
        match candle {
            Some(candle) => {
                let number_of_hours_between_adjacent_candles =
                    (candle.main.time - previous_candle.main.time).num_hours();

                assert_eq!(
                    number_of_nones_in_row,
                    number_of_hours_between_adjacent_candles - 1,
                    "the number of nons in the row ({}) should be equal to the number of hours between adjacent candles ({})",
                    number_of_nones_in_row,
                    number_of_hours_between_adjacent_candles - 1
                );

                number_of_nones_in_row = 0;
                previous_candle = candle;
            }
            None => number_of_nones_in_row += 1,
        }
    }
}

#[test]
#[ignore]
fn should_successfully_get_minute_historical_candles() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let timeframe = Timeframe::OneMin;

    let request_api = UreqRequestApi::new();

    let metaapi: MetaapiMarketDataApi<UreqRequestApi> =
        MetaapiMarketDataApi::new(auth_token, account_id, "", Default::default(), request_api);

    let end_time = Utc::now();

    let duration = Duration::weeks(4);

    let candles = metaapi.get_historical_candles(symbol, timeframe, end_time, duration);

    assert!(candles.is_ok());

    let candles = candles.unwrap();

    assert_eq!(
        candles.iter().filter(|candle| candle.is_some()).count() as i64,
        duration.num_minutes() - Duration::days(DAYS_FOR_VOLATILITY as i64).num_minutes() + 1
    );

    // test for proper amount of nones between adjacent candles
    let mut number_of_nones_in_row = 0;
    let mut previous_candle = candles.first().unwrap().as_ref().unwrap();
    for candle in &candles[1..] {
        match candle {
            Some(candle) => {
                let number_of_minutes_between_adjacent_candles =
                    (candle.main.time - previous_candle.main.time).num_minutes();

                assert_eq!(
                    number_of_nones_in_row,
                    number_of_minutes_between_adjacent_candles - 1,
                    "the number of nones in the row ({}) should be equal to the number of hours between adjacent candles ({})",
                    number_of_nones_in_row,
                    number_of_minutes_between_adjacent_candles - 1
                );

                number_of_nones_in_row = 0;
                previous_candle = candle;
            }
            None => number_of_nones_in_row += 1,
        }
    }
}

#[test]
#[ignore]
fn should_successfully_get_historical_ticks() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";

    let request_api = UreqRequestApi::new();

    let metaapi: MetaapiMarketDataApi<UreqRequestApi> =
        MetaapiMarketDataApi::new(auth_token, account_id, "", Default::default(), request_api);

    let timeframe = Timeframe::OneMin;

    let end_time = Utc::now();

    let duration = Duration::weeks(4);

    let ticks = metaapi.get_historical_ticks(symbol, timeframe, end_time, duration);

    assert!(ticks.is_ok());

    let ticks = ticks.unwrap();

    assert_eq!(
        ticks.iter().filter(|tick| tick.is_some()).count() as i64,
        duration.num_minutes() - Duration::days(DAYS_FOR_VOLATILITY as i64).num_minutes() + 1
    );

    // test for proper amount of nones between adjacent candles
    let mut number_of_nones_in_row = 0;
    let mut previous_tick = ticks.first().unwrap().as_ref().unwrap();
    for tick in &ticks[1..] {
        match tick {
            Some(tick) => {
                let number_of_minutes_between_adjacent_candles =
                    (tick.time - previous_tick.time).num_minutes();

                assert_eq!(
                    number_of_nones_in_row,
                    number_of_minutes_between_adjacent_candles - 1,
                    "the number of nones in the row ({}) should be equal to the number of hours between adjacent ticks ({})",
                    number_of_nones_in_row,
                    number_of_minutes_between_adjacent_candles - 1
                );

                number_of_nones_in_row = 0;
                previous_tick = tick;
            }
            None => number_of_nones_in_row += 1,
        }
    }
}
