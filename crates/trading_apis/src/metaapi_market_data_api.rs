use std::borrow::Cow;
use std::collections::VecDeque;
use std::marker::PhantomData;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use polars::prelude::RollingOptions;
use polars::series::Series;
use serde::Deserialize;

use base::entities::candle::{BasicCandle, CandleEdgePrice, CandleOpenClose, CandleVolatility};
use base::entities::tick::TickPrice;
use base::entities::{BasicTick, CandleBaseProperties, CandleEdgePrices, CandleType, Timeframe};
use base::helpers::{mean, price_to_points};
use base::requests::api::HttpRequest;
use base::requests::entities::{
    Headers, HttpRequestData, HttpRequestType, HttpRequestWithRetriesParams, Queries,
};
use base::requests::http_request_with_retries;

use crate::helpers::{from_iso_utc_str_to_utc_datetime, from_naive_str_to_naive_datetime};
use crate::MarketDataApi;

const MAIN_API_URL: &str = "MAIN_API_URL";
const MARKET_DATA_API_URL: &str = "MARKET_DATA_API_URL";

pub const HOURS_IN_DAY: u8 = 24;
pub const DAYS_FOR_VOLATILITY: u8 = 7;

pub type NumberOfRequestRetries = u32;
pub type SecondsToSleepBeforeRequestRetry = u32;

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
    WithoutVolatility {
        symbol: &'a str,
        timeframe: Timeframe,
    },
}

pub struct MetaapiMarketDataApi<'a, R>
where
    R: HttpRequest,
{
    auth_token: &'a str,
    account_id: &'a str,
    api_urls: ApiUrls,
    target_logger: &'a str,
    retry_settings: RetrySettings,
    request_api: &'a R,
}

impl<'a, R> MetaapiMarketDataApi<'a, R>
where
    R: HttpRequest,
{
    pub fn new(
        auth_token: &'a str,
        account_id: &'a str,
        target_logger: &'a str,
        retry_settings: RetrySettings,
        request_api: &'a R,
    ) -> MetaapiMarketDataApi<'a, R> {
        let main_url = dotenv::var(MAIN_API_URL).unwrap();
        let market_data_url = dotenv::var(MARKET_DATA_API_URL).unwrap();

        Self {
            auth_token,
            account_id,
            api_urls: ApiUrls {
                main: main_url,
                market_data: market_data_url,
            },
            target_logger,
            retry_settings,
            request_api,
        }
    }

    fn get_current_volatility(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<CandleVolatility> {
        let number_of_candles_to_determine_volatility = DAYS_FOR_VOLATILITY * HOURS_IN_DAY;

        let get_last_n_candles_url = format!(
            "{}/users/current/accounts/{}/historical-market-data/symbols/{}/timeframes/{}/candles",
            self.api_urls.market_data, self.account_id, symbol, timeframe
        );

        let limit = number_of_candles_to_determine_volatility.to_string();

        let req_data = HttpRequestData {
            req_type: HttpRequestType::Get,
            url: &get_last_n_candles_url,
            headers: Headers::from([("auth-token", self.auth_token)]),
            queries: Queries::from([("limit", limit.as_str())]),
        };

        let req_params = HttpRequestWithRetriesParams {
            req_entity_name: &format!(
                "the last {} candles",
                number_of_candles_to_determine_volatility
            ),
            target_logger: &self.target_logger,
            number_of_retries: self.retry_settings.number_of_request_retries,
            seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
        };

        let last_n_candles: Vec<MetatraderCandleJson> =
            http_request_with_retries(req_data, req_params, self.request_api)?;

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

    fn get_blocks_of_historical_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        mut total_amount_of_candles: u64,
        mut end_time: DateTime<Utc>,
    ) -> Result<VecDeque<MetatraderCandleJson>> {
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

            let start_time = end_time.to_rfc3339();
            let limit_str = limit.to_string();

            let req_data = HttpRequestData {
                req_type: HttpRequestType::Get,
                url: &get_last_n_candles_url,
                headers: Headers::from([("auth-token", self.auth_token)]),
                queries: Queries::from([
                    ("startTime", start_time.as_str()),
                    ("limit", limit_str.as_str()),
                ]),
            };

            let req_params = HttpRequestWithRetriesParams {
                req_entity_name: &format!("the block of {} candles", limit),
                target_logger: &self.target_logger,
                number_of_retries: self.retry_settings.number_of_request_retries,
                seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
            };

            let mut block_of_candles: VecDeque<MetatraderCandleJson> =
                http_request_with_retries(req_data, req_params, self.request_api)?;

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

        Ok(all_candles)
    }

    fn get_items_with_filled_gaps<T, F>(
        items: Vec<T>,
        timeframe: Timeframe,
        get_time_of_item: F,
    ) -> Result<Vec<Option<T>>>
    where
        F: Fn(&T) -> NaiveDateTime,
    {
        match items.len() {
            0 => return Ok(Vec::new()),
            1 => return Ok(items.into_iter().map(|tick| Some(tick)).collect()),
            _ => (),
        }

        let number_of_minutes_between_adjacent_items = match timeframe {
            Timeframe::Hour => 60,
            Timeframe::ThirtyMin => 30,
            Timeframe::FifteenMin => 15,
            Timeframe::OneMin => 1,
        };

        let mut all_items_with_filled_gaps: Vec<Option<T>> = Vec::new();
        let mut previous_item_time =
            get_time_of_item(items.first().context("no first tick in vector")?);

        for (i, item) in items.into_iter().enumerate() {
            let current_item_time = get_time_of_item(&item);

            if i == 0 {
                all_items_with_filled_gaps.push(Some(item));
            } else {
                let diff_in_minutes_between_current_and_previous_items =
                    (current_item_time - previous_item_time).num_minutes();

                match diff_in_minutes_between_current_and_previous_items {
                    n if n == number_of_minutes_between_adjacent_items => {
                        all_items_with_filled_gaps.push(Some(item))
                    }
                    n if n > number_of_minutes_between_adjacent_items
                        && n % number_of_minutes_between_adjacent_items == 0 =>
                    {
                        let number_of_nones_to_add = n / number_of_minutes_between_adjacent_items - 1;

                        for _ in 0..number_of_nones_to_add {
                            all_items_with_filled_gaps.push(None);
                        }

                        all_items_with_filled_gaps.push(Some(item));
                    }
                    n => bail!(
                        "invalid difference in minutes between current ({}) and previous ({}) items: {}",
                        current_item_time,
                        previous_item_time,
                        n
                    ),
                }
            }

            previous_item_time = current_item_time;
        }

        Ok(all_items_with_filled_gaps)
    }
}

impl<'a, R> MarketDataApi for MetaapiMarketDataApi<'a, R>
where
    R: HttpRequest,
{
    fn get_current_tick(&self, symbol: &str) -> Result<BasicTick> {
        let get_current_tick_url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-price",
            self.api_urls.main, self.account_id, symbol
        );

        let req_data = HttpRequestData {
            req_type: HttpRequestType::Get,
            url: &get_current_tick_url,
            headers: Headers::from([("auth-token", self.auth_token)]),
            queries: Queries::from([("keepSubscription", "true")]),
        };

        let req_params = HttpRequestWithRetriesParams {
            req_entity_name: "the current tick",
            target_logger: &self.target_logger,
            number_of_retries: self.retry_settings.number_of_request_retries,
            seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
        };

        let tick_json: MetatraderTickJson =
            http_request_with_retries(req_data, req_params, self.request_api)?;

        let time = from_naive_str_to_naive_datetime(&tick_json.broker_time)?;

        let tick = BasicTick {
            time,
            ask: tick_json.ask,
            bid: tick_json.bid,
        };

        Ok(tick)
    }

    fn get_current_candle(&self, symbol: &str, timeframe: Timeframe) -> Result<BasicCandle> {
        let get_current_candle_url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-candles/{}",
            self.api_urls.main, self.account_id, symbol, timeframe
        );

        let req_data = HttpRequestData {
            req_type: HttpRequestType::Get,
            url: &get_current_candle_url,
            headers: Headers::from([("auth-token", self.auth_token)]),
            queries: Queries::from([("keepSubscription", "true")]),
        };

        let req_params = HttpRequestWithRetriesParams {
            req_entity_name: "the current candle",
            target_logger: &self.target_logger,
            number_of_retries: self.retry_settings.number_of_request_retries,
            seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
        };

        let candle_json: MetatraderCandleJson =
            http_request_with_retries(req_data, req_params, self.request_api)?;

        self.tune_candle(
            &candle_json,
            TuneCandleConfig::WithoutVolatility { symbol, timeframe },
        )
    }

    fn get_historical_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<BasicCandle>>> {
        let days_for_volatility = Duration::days(DAYS_FOR_VOLATILITY as i64);

        let (total_amount_of_candles, volatility_window) = match timeframe {
            Timeframe::Hour => (
                duration.num_hours() as u64,
                days_for_volatility.num_hours() as usize,
            ),
            Timeframe::ThirtyMin => (
                (duration.num_hours() * 2) as u64,
                (days_for_volatility.num_hours() * 2) as usize,
            ),
            Timeframe::FifteenMin => (
                (duration.num_hours() * 4) as u64,
                (days_for_volatility.num_hours() * 4) as usize,
            ),
            Timeframe::OneMin => (
                duration.num_minutes() as u64,
                days_for_volatility.num_minutes() as usize,
            ),
        };

        let all_candles = self.get_blocks_of_historical_candles(
            symbol,
            timeframe,
            total_amount_of_candles,
            end_time,
        )?;

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

        let all_candles = all_candles
            .iter()
            .zip(all_candle_volatilities.into_iter())
            .filter(|(_, volatility)| volatility.is_some())
            .map(|(candle, volatility)| {
                self.tune_candle(
                    candle,
                    TuneCandleConfig::WithVolatility(volatility.unwrap()),
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Self::get_items_with_filled_gaps(all_candles, timeframe, |candle| candle.properties.time)
    }

    fn get_historical_ticks(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<BasicTick>>> {
        let volatility_window = Duration::days(DAYS_FOR_VOLATILITY as i64).num_minutes();
        let total_amount_of_candles = (duration.num_minutes() - volatility_window + 1) as u64;

        let all_candles = self.get_blocks_of_historical_candles(
            symbol,
            timeframe,
            total_amount_of_candles,
            end_time,
        )?;

        let all_ticks = all_candles
            .iter()
            .map(|candle| {
                Ok(BasicTick {
                    time: from_naive_str_to_naive_datetime(&candle.broker_time)?,
                    ask: candle.close,
                    bid: candle.close,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Self::get_items_with_filled_gaps(all_ticks, Timeframe::OneMin, |tick| tick.time)
    }
}
