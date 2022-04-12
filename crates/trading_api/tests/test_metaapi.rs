use log::Level;
use metaapi::{Metaapi, RetrySettings, TradingAPI};
use std::time::Instant;

#[test]
#[ignore]
fn should_successfully_get_current_tick() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";

    let metaapi = Metaapi::new(auth_token, account_id, None, Default::default());

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

    let metaapi = Metaapi::new(
        auth_token,
        account_id,
        None,
        RetrySettings {
            number_of_request_retries,
            seconds_to_sleep_before_request_retry,
        },
    );

    let start = Instant::now();
    assert!(metaapi.get_current_tick(symbol).is_err());
    let duration = start.elapsed().as_secs();

    let min_amount_of_time_for_method_execution =
        seconds_to_sleep_before_request_retry * number_of_request_retries;
    assert!(duration >= min_amount_of_time_for_method_execution as u64);

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
fn should_successfully_get_current_candle() {
    dotenv::dotenv().unwrap();

    let auth_token = dotenv::var("AUTH_TOKEN").unwrap();
    let account_id = dotenv::var("DEMO_ACCOUNT_ID").unwrap();

    let symbol = "GBPUSDm";
    let timeframe = "1h";

    let metaapi = Metaapi::new(auth_token, account_id, None, Default::default());

    println!(
        "{:?}",
        metaapi.get_current_candle(symbol, timeframe).unwrap()
    );
}

#[test]
fn should_return_an_error_after_defined_retries_of_getting_current_candle() {
    dotenv::dotenv().unwrap();

    let auth_token = String::from("invalid");
    let account_id = String::from("invalid");

    let symbol = "GBPUSDm";
    let timeframe = "1h";
    let number_of_request_retries = 3;
    let seconds_to_sleep_before_request_retry = 1;

    testing_logger::setup();

    let metaapi = Metaapi::new(
        auth_token,
        account_id,
        None,
        RetrySettings {
            number_of_request_retries,
            seconds_to_sleep_before_request_retry,
        },
    );

    let start = Instant::now();
    assert!(metaapi.get_current_candle(symbol, timeframe).is_err());
    let duration = start.elapsed().as_secs();

    let min_amount_of_time_for_method_execution =
        seconds_to_sleep_before_request_retry * number_of_request_retries;
    assert!(duration >= min_amount_of_time_for_method_execution as u64);

    testing_logger::validate(|captures_logs| {
        let number_of_error_logs = captures_logs
            .iter()
            .filter(|log| matches!(log.level, Level::Error))
            .count() as u8;
        let expected_number_of_logs = number_of_request_retries + 1;

        assert_eq!(number_of_error_logs, expected_number_of_logs);
    });
}
