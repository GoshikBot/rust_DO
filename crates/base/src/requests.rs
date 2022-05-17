use crate::requests::api::HttpRequest;
use crate::requests::entities::{HttpRequestData, HttpRequestWithRetriesParams};
use ::ureq::serde::de::DeserializeOwned;
use anyhow::{bail, Result};
use log::error;
use std::{thread, time};

pub mod api;
pub mod entities;
pub mod ureq;

pub fn http_request_with_retries<R, T>(
    req_data: HttpRequestData,
    req_params: HttpRequestWithRetriesParams,
) -> Result<T>
where
    R: HttpRequest,
    T: DeserializeOwned,
{
    let mut current_request_try = 1;

    loop {
        let response = R::call(req_data.clone());

        match response {
            Ok(item) => {
                return Ok(item);
            }
            Err(e) => {
                error!(
                    target: req_params.target_logger,
                    "an error occurred on a {} try to request {}: {:?}",
                    current_request_try, req_params.req_entity_name, e
                );

                if current_request_try <= req_params.number_of_retries {
                    thread::sleep(time::Duration::from_secs(
                        req_params.seconds_to_sleep as u64,
                    ));

                    current_request_try += 1;
                    continue;
                } else {
                    bail!(e.context(format!(
                        "an error occurred after {} retries on requesting {}",
                        req_params.number_of_retries, req_params.req_entity_name
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;
    use serde::Deserialize;
    use serde_json;
    use std::time::Instant;

    struct TestErrorRequest;

    impl HttpRequest for TestErrorRequest {
        fn call<T>(_req: HttpRequestData) -> Result<T>
        where
            T: DeserializeOwned,
        {
            bail!("error")
        }
    }

    #[derive(Deserialize)]
    struct Test {
        test: String,
    }

    struct TestSuccessfulRequest;

    impl HttpRequest for TestSuccessfulRequest {
        fn call<T>(_req: HttpRequestData) -> Result<T>
        where
            T: DeserializeOwned,
        {
            Ok(serde_json::from_str(r#"{"test": "test"}"#)?)
        }
    }

    #[test]
    #[ignore]
    fn should_return_an_error_after_request_retries() {
        let number_of_retries = 3;
        let seconds_to_sleep = 1;

        testing_logger::setup();

        let start = Instant::now();
        let res = http_request_with_retries::<TestErrorRequest, Vec<i32>>(
            Default::default(),
            HttpRequestWithRetriesParams {
                number_of_retries,
                seconds_to_sleep,
                ..Default::default()
            },
        );
        let duration = start.elapsed().as_secs();
        let min_amount_of_time_for_method_execution = (number_of_retries * seconds_to_sleep) as u64;

        assert!(
            duration >= min_amount_of_time_for_method_execution,
            "execution time of the function ({}) should be >= than min amount of time ({})",
            duration,
            min_amount_of_time_for_method_execution
        );

        assert!(
            res.is_err(),
            "the request should be completed with an error"
        );

        // check that the defined number of error log messages
        // for the request retries were called
        testing_logger::validate(|captures_logs| {
            let number_of_error_logs = captures_logs
                .iter()
                .filter(|log| matches!(log.level, Level::Error))
                .count() as u32;
            let expected_number_of_logs = number_of_retries + 1;

            assert_eq!(
                number_of_error_logs, expected_number_of_logs,
                "the number of error logs ({}) should be equal to expected amount ({})",
                number_of_error_logs, expected_number_of_logs
            );
        });
    }

    #[test]
    fn should_successfully_request_item() {
        let res = http_request_with_retries::<TestSuccessfulRequest, Test>(
            Default::default(),
            Default::default(),
        );

        assert!(res.is_ok());
        assert_eq!(res.unwrap().test, "test");
    }
}
