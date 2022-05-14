use chrono::{DateTime, Duration, Timelike};
use log::Level;
use std::time::Instant;
use trading_apis::entities::HistoricalTimeframe;
use trading_apis::{MarketDataApi, MetaapiMarketDataApi, RetrySettings};

#[test]
#[ignore]
fn should_successfully_get_current_tick() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        String::from("test"),
        Default::default(),
    );

    assert!(metaapi.get_current_tick(symbol).is_ok());
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

    testing_logger::setup();

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        String::from("test"),
        RetrySettings {
            number_of_request_retries,
            seconds_to_sleep_before_request_retry,
        },
    );

    // check that the method takes at least min amount of time to execute
    // to make sure the retries of a request work
    let start = Instant::now();
    assert!(metaapi.get_current_tick(symbol).is_err());
    let duration = start.elapsed().as_secs();

    let min_amount_of_time_for_method_execution =
        seconds_to_sleep_before_request_retry * number_of_request_retries;
    assert!(duration >= min_amount_of_time_for_method_execution as u64);

    // check that the defined number of error log messages
    // for the request retries were called
    testing_logger::validate(|captures_logs| {
        let number_of_error_logs = captures_logs
            .iter()
            .filter(|log| matches!(log.level, Level::Error))
            .count() as u8;
        let expected_number_of_logs = number_of_request_retries + 1;

        assert_eq!(number_of_error_logs, expected_number_of_logs);
    });
}

#[test]
#[ignore]
fn should_successfully_get_current_candle() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let timeframe = "1h";

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        String::from("test"),
        Default::default(),
    );

    assert!(metaapi.get_current_candle(symbol, timeframe).is_ok());
}

#[test]
#[ignore]
fn should_return_an_error_after_defined_retries_of_getting_current_candle() {
    dotenv::dotenv().unwrap();

    let auth_token = String::from("invalid");
    let account_id = String::from("invalid");

    let symbol = "GBPUSDm";
    let timeframe = "1h";
    let number_of_request_retries = 3;
    let seconds_to_sleep_before_request_retry = 1;

    testing_logger::setup();

    let metaapi = MetaapiMarketDataApi::new(
        auth_token,
        account_id,
        String::from("test"),
        RetrySettings {
            number_of_request_retries,
            seconds_to_sleep_before_request_retry,
        },
    );

    // check that the method takes at least min amount of time to execute
    // to make sure the retries of a request work
    let start = Instant::now();
    assert!(metaapi.get_current_candle(symbol, timeframe).is_err());
    let duration = start.elapsed().as_secs();

    let min_amount_of_time_for_method_execution =
        seconds_to_sleep_before_request_retry * number_of_request_retries;
    assert!(duration >= min_amount_of_time_for_method_execution as u64);

    // check that the defined number of error log messages
    // for the request retries were called
    testing_logger::validate(|captures_logs| {
        let number_of_error_logs = captures_logs
            .iter()
            .filter(|log| matches!(log.level, Level::Error))
            .count() as u8;
        let expected_number_of_logs = number_of_request_retries + 1;

        assert_eq!(number_of_error_logs, expected_number_of_logs);
    });
}

#[test]
#[ignore]
fn should_successfully_get_historical_candles() {
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

    let candles = metaapi.get_historical_candles(symbol, timeframe, end_time, duration);

    assert!(candles.is_ok());

    let candles = candles.unwrap();

    let diff_between_first_and_last_candles =
        candles.last().unwrap().properties.time - candles.first().unwrap().properties.time;
    assert!(diff_between_first_and_last_candles >= duration);

    // checks that there are no skipped candles
    let mut current_hour = candles.first().unwrap().properties.time.hour();
    let mut expected_hour = match current_hour {
        23 => 0,
        _ => current_hour + 1,
    };

    for candle in (&candles[1..]).iter() {
        current_hour = candle.properties.time.hour();

        assert_eq!(current_hour, expected_hour);

        expected_hour = match current_hour {
            23 => 0,
            _ => current_hour + 1,
        }
    }
}
