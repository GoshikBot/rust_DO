use std::{thread, time};

use anyhow::{Context, Result};
use serde::Deserialize;
use ureq::Response;

use base::entities::candle::{BasicCandle, CandleEdgePrice, CandleOpenClose, CandleVolatility};
use base::entities::tick::TickPrice;
use base::entities::{CandleBaseProperties, CandleEdgePrices, CandleType, TickBaseProperties};
use base::helpers::{mean, price_to_points};

use crate::base_api::TradingAPI;
use crate::helpers::{log_message, to_time, LogLevel};

type DaysForVolatility = u8;
pub type NumberOfRequestRetries = u8;
type SecondsToSleepBeforeRequestRetry = u8;

pub const HOURS_IN_DAY: u8 = 24;
pub const DAYS_FOR_VOLATILITY: DaysForVolatility = 7;

pub const DEFAULT_NUMBER_OF_REQUEST_RETRIES: NumberOfRequestRetries = 5;
pub const DEFAULT_NUMBER_OF_SECONDS_TO_SLEEP_BEFORE_REQUEST_RETRY:
    SecondsToSleepBeforeRequestRetry = 1;

type MetatraderBrokerTime = String;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MetatraderTickJson {
    broker_time: MetatraderBrokerTime,
    ask: TickPrice,
    bid: TickPrice,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MetatraderCandleJson {
    broker_time: MetatraderBrokerTime,
    open: CandleEdgePrice,
    high: CandleEdgePrice,
    low: CandleEdgePrice,
    close: CandleEdgePrice,
}

enum HttpMethod {
    Get,
}

pub struct RetrySettings {
    pub number_of_request_retries: NumberOfRequestRetries,
    pub seconds_to_sleep_before_request_retry: SecondsToSleepBeforeRequestRetry,
}

pub type AuthToken = String;
pub type AccountId = String;
pub type Symbol = String;
pub type ApiUrl = String;
pub type Timeframe = String;
pub type LoggerTarget = String;

struct ApiUrls {
    main: ApiUrl,
    market_data: ApiUrl,
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            number_of_request_retries: DEFAULT_NUMBER_OF_REQUEST_RETRIES,
            seconds_to_sleep_before_request_retry:
                DEFAULT_NUMBER_OF_SECONDS_TO_SLEEP_BEFORE_REQUEST_RETRY,
        }
    }
}

pub struct Metaapi {
    auth_token: AuthToken,
    account_id: AccountId,
    api_urls: ApiUrls,
    logger_target: Option<LoggerTarget>,
    retry_settings: RetrySettings,
}

impl Metaapi {
    pub fn new(
        auth_token: AuthToken,
        account_id: AccountId,
        logger_target: Option<LoggerTarget>,
        retry_settings: RetrySettings,
    ) -> Metaapi {
        let main_api_url = dotenv::var("MAIN_API_URL").unwrap();
        let market_data_url = dotenv::var("MARKET_DATA_API_URL").unwrap();

        Metaapi {
            auth_token,
            account_id,
            api_urls: ApiUrls {
                main: main_api_url,
                market_data: market_data_url,
            },
            logger_target,
            retry_settings,
        }
    }

    fn request_with_retries(
        &self,
        url: &str,
        request_entity_name: &str,
        method: HttpMethod,
        keep_subscription: Option<bool>,
    ) -> Result<Response> {
        let mut current_request_try = 1;

        loop {
            let mut response = match method {
                HttpMethod::Get => ureq::get(url).set("auth-token", &self.auth_token),
            };

            if let Some(keep_subscription) = keep_subscription {
                response = response.query(
                    "keepSubscription",
                    if keep_subscription { "true" } else { "false" },
                );
            }

            let response = response.call();

            match response {
                Ok(item) => {
                    break Ok(item);
                }
                Err(err) => {
                    log_message(
                        &format!(
                            "an error occurred on a {} try to request {}: {:?}",
                            current_request_try, request_entity_name, err
                        ),
                        LogLevel::Error,
                        self.logger_target.as_deref(),
                    );

                    if current_request_try <= self.retry_settings.number_of_request_retries {
                        thread::sleep(time::Duration::from_secs(
                            self.retry_settings.seconds_to_sleep_before_request_retry as u64,
                        ));

                        current_request_try += 1;
                        continue;
                    } else {
                        return Err(err).context(format!(
                            "an error occurred on requesting {}",
                            request_entity_name
                        ));
                    }
                }
            }
        }
    }

    fn get_current_volatility(&self, symbol: &str, timeframe: &str) -> Result<CandleVolatility> {
        let number_of_candles_to_determine_volatility = DAYS_FOR_VOLATILITY * HOURS_IN_DAY;

        let get_last_n_candles_url = format!(
            "{}/users/current/accounts/{}/historical-market-data/symbols/{}/timeframes/{}/candles",
            self.api_urls.market_data, self.account_id, symbol, timeframe
        );

        let response = self.request_with_retries(
            &get_last_n_candles_url,
            &format!(
                "the last {} candles",
                number_of_candles_to_determine_volatility
            ),
            HttpMethod::Get,
            None,
        )?;

        let last_n_candles: Vec<MetatraderCandleJson> = response.into_json().context(format!(
            "an error occurred on parsing the last {} candles response to the inner struct",
            number_of_candles_to_determine_volatility
        ))?;

        let sizes_of_candles: Vec<f32> = last_n_candles
            .iter()
            .map(|candle| price_to_points(candle.high - candle.low))
            .collect();

        Ok(mean(&sizes_of_candles))
    }

    fn tune_candle(
        &self,
        candle_json: MetatraderCandleJson,
        symbol: &str,
        timeframe: &str,
    ) -> Result<BasicCandle> {
        let candle_edge_prices = CandleEdgePrices {
            open: candle_json.open,
            high: candle_json.high,
            low: candle_json.low,
            close: candle_json.close,
        };

        let current_volatility = self.get_current_volatility(symbol, timeframe)?;

        let candle_size = candle_json.high - candle_json.low;

        let candle_type = CandleType::from(CandleOpenClose {
            open: candle_json.open,
            close: candle_json.close,
        });

        let candle_time = to_time(&candle_json.broker_time)?;

        let candle_base_properties = CandleBaseProperties {
            time: candle_time,
            size: candle_size,
            r#type: candle_type,
            volatility: current_volatility,
        };

        Ok(BasicCandle {
            base_properties: candle_base_properties,
            edge_prices: candle_edge_prices,
        })
    }
}

impl TradingAPI for Metaapi {
    fn get_current_tick(&self, symbol: &str) -> Result<TickBaseProperties> {
        let url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-price",
            self.api_urls.main, self.account_id, symbol
        );

        let response = self
            .request_with_retries(&url, "the current_tick", HttpMethod::Get, Some(true))
            .context(format!(
                "wasn't able to get the current tick after {} retries",
                self.retry_settings.number_of_request_retries
            ))?;

        let tick_json: MetatraderTickJson = response
            .into_json()
            .context("an error occurred on parsing the current tick response to an inner struct")?;

        let time = to_time(&tick_json.broker_time)?;

        let tick = TickBaseProperties {
            time,
            ask: tick_json.ask,
            bid: tick_json.bid,
        };

        Ok(tick)
    }

    fn get_current_candle(&self, symbol: &str, timeframe: &str) -> Result<BasicCandle> {
        let get_current_candle_url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-candles/{}",
            self.api_urls.main, self.account_id, symbol, timeframe
        );

        let response = self
            .request_with_retries(
                &get_current_candle_url,
                "the current candle",
                HttpMethod::Get,
                Some(true),
            )
            .context(format!(
                "wasn't able to get the current candle after {} retries",
                self.retry_settings.number_of_request_retries
            ))?;

        let candle_json: MetatraderCandleJson = response.into_json().context(
            "an error occurred on parsing the current candle response to the inner struct",
        )?;

        self.tune_candle(candle_json, symbol, timeframe)
    }
}
