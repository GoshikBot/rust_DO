use crate::entities::TARGET_LOGGER_ENV;
use crate::requests::api::SyncHttpRequest;
use crate::requests::entities::{HttpRequestData, HttpRequestWithRetriesParams};
use ::ureq::serde::de::DeserializeOwned;
use anyhow::{bail, Result};
use std::{thread, time};

pub mod api;
pub mod entities;
pub mod ureq;

pub fn http_request_with_retries(
    req_data: HttpRequestData,
    req_params: HttpRequestWithRetriesParams,
    request_api: &impl SyncHttpRequest,
) -> Result<String> {
    let mut current_request_try = 1;

    loop {
        let response = request_api.call(req_data.clone());

        match response {
            Ok(item) => {
                return Ok(item);
            }
            Err(e) => {
                log::error!(
                    target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
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
    use serde::Deserialize;
    use serde_json;
    use std::cell::RefCell;
    use std::time::Instant;

    #[derive(Default)]
    struct HttpErrorRequest {
        number_of_requests: RefCell<u32>,
    }

    impl SyncHttpRequest for HttpErrorRequest {
        fn call(&self, _req: HttpRequestData) -> Result<String> {
            *self.number_of_requests.borrow_mut() += 1;
            bail!("error")
        }
    }

    #[derive(Default)]
    struct HttpSuccessfulRequest {
        number_of_requests: RefCell<u32>,
    }

    impl SyncHttpRequest for HttpSuccessfulRequest {
        fn call(&self, _req: HttpRequestData) -> Result<String> {
            *self.number_of_requests.borrow_mut() += 1;
            Ok(String::from("success"))
        }
    }

    #[test]
    #[ignore]
    fn should_return_an_error_after_request_retries() {
        let number_of_retries = 3;
        let seconds_to_sleep = 1;

        let http_request: HttpErrorRequest = Default::default();

        let start = Instant::now();
        let res = http_request_with_retries(
            Default::default(),
            HttpRequestWithRetriesParams {
                number_of_retries,
                seconds_to_sleep,
                ..Default::default()
            },
            &http_request,
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

        let expected_number_of_requests = number_of_retries + 1;
        assert_eq!(
            *http_request.number_of_requests.borrow(),
            expected_number_of_requests
        );
    }

    #[test]
    fn should_successfully_request_item() {
        let http_request: HttpSuccessfulRequest = Default::default();

        let res = http_request_with_retries(Default::default(), Default::default(), &http_request);

        assert!(res.is_ok());
        assert_eq!(*http_request.number_of_requests.borrow(), 1);
        assert_eq!(res.unwrap(), "success");
    }
}
