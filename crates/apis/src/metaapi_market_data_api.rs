use std::collections::{LinkedList, VecDeque};
use std::{thread, time};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use log::{error, info};
use polars::prelude::RollingOptions;
use polars::series::Series;
use serde::Deserialize;
use ureq::{Request, Response};

use base::entities::candle::{BasicCandle, CandleEdgePrice, CandleOpenClose, CandleVolatility};
use base::entities::tick::TickPrice;
use base::entities::{CandleBaseProperties, CandleEdgePrices, CandleType, TickBaseProperties};
use base::helpers::{mean, price_to_points};

use crate::api::MarketDataApi;
use crate::entities::HistoricalTimeframe;
use crate::helpers::{
    from_iso_utc_str_to_utc_datetime, from_naive_str_to_naive_datetime,
    get_amount_of_weekends_between_two_dates,
};

pub const HOURS_IN_DAY: u8 = 24;
pub const DAYS_FOR_VOLATILITY: u8 = 7;

pub type NumberOfRequestRetries = u8;
pub type SecondsToSleepBeforeRequestRetry = u8;

pub const DEFAULT_NUMBER_OF_REQUEST_RETRIES: NumberOfRequestRetries = 5;
pub const DEFAULT_NUMBER_OF_SECONDS_TO_SLEEP_BEFORE_REQUEST_RETRY:
    SecondsToSleepBeforeRequestRetry = 1;

const MAX_NUMBER_OF_CANDLES_PER_REQUEST: u64 = 1000;

type MetatraderTime = String;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MetatraderTickJson {
    broker_time: MetatraderTime,
    ask: TickPrice,
    bid: TickPrice,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MetatraderCandleJson {
    time: MetatraderTime,
    broker_time: MetatraderTime,
    open: CandleEdgePrice,
    high: CandleEdgePrice,
    low: CandleEdgePrice,
    close: CandleEdgePrice,
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

enum TuneCandleConfig<'a> {
    WithVolatility(CandleVolatility),
    WithoutVolatility { symbol: &'a str, timeframe: &'a str },
}

pub struct MetaapiMarketDataApi {
    auth_token: AuthToken,
    account_id: AccountId,
    api_urls: ApiUrls,
    target_logger: LoggerTarget,
    retry_settings: RetrySettings,
}

impl MetaapiMarketDataApi {
    pub fn new(
        auth_token: AuthToken,
        account_id: AccountId,
        logger_target: LoggerTarget,
        retry_settings: RetrySettings,
    ) -> MetaapiMarketDataApi {
        let main_url = dotenv::var("MAIN_API_URL").unwrap();
        let market_data_url = dotenv::var("MARKET_DATA_API_URL").unwrap();

        MetaapiMarketDataApi {
            auth_token,
            account_id,
            api_urls: ApiUrls {
                main: main_url,
                market_data: market_data_url,
            },
            target_logger: logger_target,
            retry_settings,
        }
    }

    fn request_with_retries(
        &self,
        request: Request,
        request_entity_name: &str,
    ) -> Result<Response> {
        let mut current_request_try = 1;

        loop {
            let response = request.clone().call();

            match response {
                Ok(item) => {
                    break Ok(item);
                }
                Err(err) => {
                    error!(
                        target: &self.target_logger,
                        "an error occurred on a {} try to request {}: {:?}",
                        current_request_try, request_entity_name, err
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

        let request = ureq::get(&get_last_n_candles_url)
            .set("auth-token", &self.auth_token)
            .query(
                "limit",
                &number_of_candles_to_determine_volatility.to_string(),
            );

        assert!(
            format!("{:?}", request).contains(&format!(
                "limit={}",
                number_of_candles_to_determine_volatility
            )),
            "amount of candles to determine volatility is wrong"
        );

        let response = self.request_with_retries(
            request,
            &format!(
                "the last {} candles",
                number_of_candles_to_determine_volatility
            ),
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
        candle_json: &MetatraderCandleJson,
        config: TuneCandleConfig,
    ) -> Result<BasicCandle> {
        let candle_edge_prices = CandleEdgePrices {
            open: candle_json.open,
            high: candle_json.high,
            low: candle_json.low,
            close: candle_json.close,
        };

        let current_volatility = match config {
            TuneCandleConfig::WithoutVolatility { symbol, timeframe } => {
                self.get_current_volatility(symbol, timeframe)?
            }
            TuneCandleConfig::WithVolatility(volatility) => volatility,
        };

        let candle_size = candle_json.high - candle_json.low;

        let candle_type = CandleType::from(CandleOpenClose {
            open: candle_json.open,
            close: candle_json.close,
        });

        let candle_time = from_naive_str_to_naive_datetime(&candle_json.broker_time)?;

        let candle_base_properties = CandleBaseProperties {
            time: candle_time,
            size: candle_size,
            r#type: candle_type,
            volatility: current_volatility,
        };

        Ok(BasicCandle {
            properties: candle_base_properties,
            edge_prices: candle_edge_prices,
        })
    }
}

impl MarketDataApi for MetaapiMarketDataApi {
    fn get_current_tick(&self, symbol: &str) -> Result<TickBaseProperties> {
        let url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-price",
            self.api_urls.main, self.account_id, symbol
        );

        let request = ureq::get(&url)
            .set("auth-token", &self.auth_token)
            .query("keepSubscription", "true");

        let response = self
            .request_with_retries(request, "the current tick")
            .context(format!(
                "wasn't able to get the current tick after {} retries",
                self.retry_settings.number_of_request_retries
            ))?;

        let tick_json: MetatraderTickJson = response
            .into_json()
            .context("an error occurred on parsing the current tick response to an inner struct")?;

        let time = from_naive_str_to_naive_datetime(&tick_json.broker_time)?;

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

        let request = ureq::get(&get_current_candle_url)
            .set("auth-token", &self.auth_token)
            .query("keepSubscription", "true");

        let response = self
            .request_with_retries(request, "the current candle")
            .context(format!(
                "wasn't able to get the current candle after {} retries",
                self.retry_settings.number_of_request_retries
            ))?;

        let candle_json: MetatraderCandleJson = response.into_json().context(
            "an error occurred on parsing the current candle response to the inner struct",
        )?;

        self.tune_candle(
            &candle_json,
            TuneCandleConfig::WithoutVolatility { symbol, timeframe },
        )
    }

    fn get_historical_candles(
        &self,
        symbol: &str,
        timeframe: HistoricalTimeframe,
        mut end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<BasicCandle>> {
        let days_for_volatility = Duration::days(DAYS_FOR_VOLATILITY as i64);

        let (mut total_amount_of_candles, volatility_window) = match timeframe {
            HistoricalTimeframe::Hour => (
                duration.num_hours() as u64,
                days_for_volatility.num_hours() as usize,
            ),
            HistoricalTimeframe::ThirtyMin => (
                (duration.num_hours() * 2) as u64,
                (days_for_volatility.num_hours() * 2) as usize,
            ),
            HistoricalTimeframe::FifteenMin => (
                (duration.num_hours() * 4) as u64,
                (days_for_volatility.num_hours() * 4) as usize,
            ),
            HistoricalTimeframe::OneMin => (
                duration.num_minutes() as u64,
                days_for_volatility.num_minutes() as usize,
            ),
        };

        let get_last_n_candles_url = format!(
            "{}/users/current/accounts/{}/historical-market-data/symbols/{}/timeframes/{}/candles",
            self.api_urls.market_data, self.account_id, symbol, timeframe
        );

        let mut all_candles = VecDeque::new();

        while total_amount_of_candles > 0 {
            let limit = if total_amount_of_candles > MAX_NUMBER_OF_CANDLES_PER_REQUEST {
                MAX_NUMBER_OF_CANDLES_PER_REQUEST
            } else {
                total_amount_of_candles
            };

            let request = ureq::get(&get_last_n_candles_url)
                .set("auth-token", &self.auth_token)
                .query("startTime", &end_time.to_rfc3339())
                .query("limit", &limit.to_string());

            let response = self
                .request_with_retries(request, &format!("the block of {} candles", limit))
                .context(format!(
                    "wasn't able to get historical candles after {} retries",
                    self.retry_settings.number_of_request_retries
                ))?;

            let mut block_of_candles: VecDeque<MetatraderCandleJson> = response.into_json().context(
                "an error occurred on parsing the block of historical candles response to the inner struct",
            )?;

            block_of_candles.append(&mut all_candles);
            all_candles = block_of_candles;

            total_amount_of_candles -= if limit == MAX_NUMBER_OF_CANDLES_PER_REQUEST {
                limit - 1
            } else {
                limit
            };

            if total_amount_of_candles != 0 {
                end_time =
                    from_iso_utc_str_to_utc_datetime(&all_candles.pop_front().unwrap().time)?;
            }
        }

        let all_candle_sizes: Series = all_candles
            .iter()
            .map(|candle| price_to_points(candle.high - candle.low))
            .collect();

        let all_candle_volatilities = all_candle_sizes
            .rolling_mean(RollingOptions {
                window_size: volatility_window,
                min_periods: volatility_window,
                weights: None,
                center: false,
            })
            .context("error on rolling candle volatilities")?;

        let all_candle_volatilities = all_candle_volatilities
            .f32()
            .context("error on casting rolling volatilities to f32 ChunkedArray")?;

        all_candles
            .iter()
            .zip(all_candle_volatilities.into_iter())
            .filter(|(_, volatility)| volatility.is_some())
            .map(|(candle, volatility)| {
                self.tune_candle(
                    candle,
                    TuneCandleConfig::WithVolatility(volatility.unwrap()),
                )
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
