use std::collections::VecDeque;
use std::{thread, time};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use polars::prelude::RollingOptions;
use polars::series::Series;
use rust_decimal::Decimal;
use serde::Deserialize;
use ureq::serde_json;

use base::entities::candle::{BasicCandleProperties, CandlePrice, CandleVolatility};
use base::entities::tick::TickPrice;
use base::entities::{BasicTickProperties, CandlePrices, CandleType, Timeframe};
use base::helpers::{mean, price_to_points};
use base::requests::api::SyncHttpRequest;
use base::requests::entities::{HttpRequestData, HttpRequestMethod, HttpRequestWithRetriesParams};
use base::requests::http_request_with_retries;

use crate::helpers::{from_iso_utc_str_to_utc_datetime, from_naive_str_to_naive_datetime};
use crate::MarketDataApi;

pub const AUTH_TOKEN_ENV: &str = "AUTH_TOKEN";
pub const DEMO_ACCOUNT_ID_ENV: &str = "DEMO_ACCOUNT_ID";
pub const MAIN_API_URL_ENV: &str = "MAIN_API_URL";
pub const MARKET_DATA_API_URL_ENV: &str = "MARKET_DATA_API_URL";

pub const HOURS_IN_DAY: u8 = 24;
pub const DAYS_FOR_VOLATILITY: u8 = 7;

pub type NumberOfRequestRetries = u32;
pub type SecondsToSleepBeforeRequestRetry = u32;

pub const DEFAULT_NUMBER_OF_REQUEST_RETRIES: NumberOfRequestRetries = 5;
pub const DEFAULT_NUMBER_OF_SECONDS_TO_SLEEP_BEFORE_REQUEST_RETRY:
    SecondsToSleepBeforeRequestRetry = 1;

const MAX_NUMBER_OF_CANDLES_PER_REQUEST: u64 = 1000;

const SECONDS_TO_SLEEP_AFTER_BLOCK_REQUEST: u8 = 1;

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
    open: CandlePrice,
    high: CandlePrice,
    low: CandlePrice,
    close: CandlePrice,
}

pub struct RetrySettings {
    pub number_of_request_retries: NumberOfRequestRetries,
    pub seconds_to_sleep_before_request_retry: SecondsToSleepBeforeRequestRetry,
}

pub type AuthToken = String;
pub type AccountId = String;
pub type Symbol = String;
pub type ApiUrl = String;
pub type TargetLogger = String;

#[derive(Default)]
pub struct ApiUrls {
    pub main: ApiUrl,
    pub market_data: ApiUrl,
}

#[derive(Default)]
pub struct ApiData {
    pub auth_token: AuthToken,
    pub account_id: AccountId,
    pub urls: ApiUrls,
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

pub struct MetaapiMarketDataApi<R>
where
    R: SyncHttpRequest,
{
    api_data: ApiData,
    retry_settings: RetrySettings,
    request_api: R,
}

impl<R: SyncHttpRequest> MetaapiMarketDataApi<R> {
    pub fn new(
        api_data: ApiData,
        retry_settings: RetrySettings,
        request_api: R,
    ) -> MetaapiMarketDataApi<R> {
        Self {
            api_data,
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
            self.api_data.urls.market_data, self.api_data.account_id, symbol, timeframe
        );

        let limit = number_of_candles_to_determine_volatility.to_string();

        let req_data = HttpRequestData::new(HttpRequestMethod::Get, get_last_n_candles_url)
            .add_header("auth-token", &self.api_data.auth_token)
            .add_query("limit", limit);

        let req_params = HttpRequestWithRetriesParams {
            req_entity_name: &format!(
                "the last {} candles",
                number_of_candles_to_determine_volatility
            ),
            number_of_retries: self.retry_settings.number_of_request_retries,
            seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
        };

        let last_n_candles: Vec<MetatraderCandleJson> = serde_json::from_str(
            &http_request_with_retries(req_data, req_params, &self.request_api)?,
        )?;

        let sizes_of_candles: Vec<_> = last_n_candles
            .iter()
            .map(|candle| price_to_points(candle.high - candle.low))
            .collect();

        Ok(mean(&sizes_of_candles)
            .round()
            .to_string()
            .parse::<CandleVolatility>()
            .unwrap())
    }

    fn get_all_volatilities(
        &self,
        candles: &[MetatraderCandleJson],
        window: usize,
    ) -> Result<Vec<Option<CandleVolatility>>> {
        let candle_sizes: Series = candles
            .iter()
            .map(|candle| {
                price_to_points(candle.high - candle.low)
                    .to_string()
                    .parse::<f32>()
                    .unwrap()
            })
            .collect();

        let all_candle_volatilities = candle_sizes
            .rolling_mean(RollingOptions {
                window_size: window,
                min_periods: window,
                weights: None,
                center: false,
            })
            .context("error on rolling candle volatilities")?;

        let candle_volatilities = all_candle_volatilities
            .f32()
            .context("error on casting rolling volatilities to f32 ChunkedArray")?
            .into_iter()
            .map(|volatility| {
                volatility.map(|value| {
                    Decimal::try_from(value)
                        .unwrap()
                        .round()
                        .to_string()
                        .parse::<CandleVolatility>()
                        .unwrap()
                })
            })
            .collect();

        Ok(candle_volatilities)
    }

    fn tune_candle(
        &self,
        candle_json: &MetatraderCandleJson,
        current_volatility: CandleVolatility,
    ) -> Result<BasicCandleProperties> {
        let candle_edge_prices = CandlePrices {
            open: candle_json.open,
            high: candle_json.high,
            low: candle_json.low,
            close: candle_json.close,
        };

        let candle_size = price_to_points(candle_json.high - candle_json.low);

        let candle_type = CandleType::from(&candle_edge_prices);

        let candle_time = from_naive_str_to_naive_datetime(&candle_json.broker_time)?;

        Ok(BasicCandleProperties {
            time: candle_time,
            size: candle_size,
            r#type: candle_type,
            volatility: current_volatility,
            prices: candle_edge_prices,
        })
    }

    fn get_blocks_of_historical_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        mut total_amount_of_candles: u64,
        mut end_time: DateTime<Utc>,
    ) -> Result<Vec<MetatraderCandleJson>> {
        let get_last_n_candles_url = format!(
            "{}/users/current/accounts/{}/historical-market-data/symbols/{}/timeframes/{}/candles",
            self.api_data.urls.market_data, self.api_data.account_id, symbol, timeframe
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

            let req_data = HttpRequestData::new(HttpRequestMethod::Get, &get_last_n_candles_url)
                .add_header("auth-token", &self.api_data.auth_token)
                .add_query("limit", limit_str)
                .add_query("startTime", start_time);

            let req_params = HttpRequestWithRetriesParams {
                req_entity_name: &format!("the block of {} candles", limit),
                number_of_retries: self.retry_settings.number_of_request_retries,
                seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
            };

            let mut block_of_candles: VecDeque<MetatraderCandleJson> = serde_json::from_str(
                &http_request_with_retries(req_data, req_params, &self.request_api)?,
            )?;

            thread::sleep(time::Duration::from_secs(
                SECONDS_TO_SLEEP_AFTER_BLOCK_REQUEST as u64,
            ));

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

        Ok(all_candles.into_iter().collect())
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
            Timeframe::FiveMin => 5,
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

impl<R: SyncHttpRequest> MarketDataApi for MetaapiMarketDataApi<R> {
    fn get_current_tick(&self, symbol: &str) -> Result<BasicTickProperties> {
        let get_current_tick_url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-price",
            self.api_data.urls.main, self.api_data.account_id, symbol
        );

        let req_data = HttpRequestData::new(HttpRequestMethod::Get, get_current_tick_url)
            .add_header("auth-token", &self.api_data.auth_token)
            .add_query("keepSubscription", "true");

        let req_params = HttpRequestWithRetriesParams {
            req_entity_name: "the current tick",
            number_of_retries: self.retry_settings.number_of_request_retries,
            seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
        };

        let tick_json: MetatraderTickJson = serde_json::from_str(&http_request_with_retries(
            req_data,
            req_params,
            &self.request_api,
        )?)?;

        let time = from_naive_str_to_naive_datetime(&tick_json.broker_time)?;

        let tick = BasicTickProperties {
            time,
            ask: tick_json.ask,
            bid: tick_json.bid,
        };

        Ok(tick)
    }

    fn get_current_candle(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<BasicCandleProperties> {
        let get_current_candle_url = format!(
            "{}/users/current/accounts/{}/symbols/{}/current-candles/{}",
            self.api_data.urls.main, self.api_data.account_id, symbol, timeframe
        );

        let req_data = HttpRequestData::new(HttpRequestMethod::Get, get_current_candle_url)
            .add_header("auth-token", &self.api_data.auth_token)
            .add_query("keepSubscription", "true");

        let req_params = HttpRequestWithRetriesParams {
            req_entity_name: "the current candle",
            number_of_retries: self.retry_settings.number_of_request_retries,
            seconds_to_sleep: self.retry_settings.seconds_to_sleep_before_request_retry,
        };

        let candle_json: MetatraderCandleJson = serde_json::from_str(&http_request_with_retries(
            req_data,
            req_params,
            &self.request_api,
        )?)?;

        let current_volatility = self.get_current_volatility(symbol, timeframe)?;
        self.tune_candle(&candle_json, current_volatility)
    }

    fn get_historical_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<BasicCandleProperties>>> {
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
            Timeframe::FiveMin => (
                (duration.num_minutes() / 5) as u64,
                (days_for_volatility.num_minutes() / 5) as usize,
            ),
        };

        let all_candles = self.get_blocks_of_historical_candles(
            symbol,
            timeframe,
            total_amount_of_candles,
            end_time,
        )?;

        let all_candle_volatilities = self.get_all_volatilities(&all_candles, volatility_window)?;

        let all_candles = all_candles
            .iter()
            .zip(all_candle_volatilities.into_iter())
            .filter(|(_, volatility)| volatility.is_some())
            .map(|(candle, volatility)| {
                self.tune_candle(
                    candle,
                    Decimal::try_from(volatility.unwrap())
                        .unwrap()
                        .round()
                        .to_string()
                        .parse::<u32>()
                        .unwrap(),
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Self::get_items_with_filled_gaps(all_candles, timeframe, |candle| candle.time)
    }

    fn get_historical_ticks(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<BasicTickProperties>>> {
        let days_for_volatility = Duration::days(DAYS_FOR_VOLATILITY as i64);

        let total_amount_of_candles = match timeframe {
            Timeframe::Hour => (duration.num_hours() - days_for_volatility.num_hours()) as u64,
            Timeframe::ThirtyMin => {
                ((duration.num_hours() * 2) - (days_for_volatility.num_hours() * 2)) as u64
            }
            Timeframe::FifteenMin => {
                ((duration.num_hours() * 4) - (days_for_volatility.num_hours() * 4)) as u64
            }
            Timeframe::OneMin => {
                (duration.num_minutes() - days_for_volatility.num_minutes()) as u64
            }
            Timeframe::FiveMin => {
                ((duration.num_minutes() / 5) - (days_for_volatility.num_minutes() / 5)) as u64
            }
        } + 1;

        let all_candles = self.get_blocks_of_historical_candles(
            symbol,
            timeframe,
            total_amount_of_candles,
            end_time,
        )?;

        let all_ticks = all_candles
            .iter()
            .map(|candle| {
                Ok(BasicTickProperties {
                    time: from_naive_str_to_naive_datetime(&candle.broker_time)?,
                    ask: candle.close,
                    bid: candle.close,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Self::get_items_with_filled_gaps(all_ticks, timeframe, |tick| tick.time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    struct TestRequestApi;

    impl SyncHttpRequest for TestRequestApi {
        fn call(&self, _req: HttpRequestData) -> Result<String> {
            Ok(r#"[
  {
    "time": "2022-06-21T10:00:00.000Z",
    "open": 1.22958,
    "high": 1.23006,
    "low": 1.22781,
    "close": 1.22806,
    "brokerTime": "2022-06-21 13:00:00.000"
  },
  {
    "time": "2022-06-21T11:00:00.000Z",
    "open": 1.22805,
    "high": 1.22863,
    "low": 1.22507,
    "close": 1.22685,
    "brokerTime": "2022-06-21 14:00:00.000"
  },
  {
    "time": "2022-06-21T12:00:00.000Z",
    "open": 1.22686,
    "high": 1.22812,
    "low": 1.22596,
    "close": 1.22662,
    "brokerTime": "2022-06-21 15:00:00.000"
  },
  {
    "time": "2022-06-21T13:00:00.000Z",
    "open": 1.22664,
    "high": 1.22943,
    "low": 1.22655,
    "close": 1.22857,
    "brokerTime": "2022-06-21 16:00:00.000"
  }
]"#
            .to_string())
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_current_volatility__should_return_correct_value() {
        let symbol = "smth";

        let request_api = TestRequestApi {};

        let metaapi =
            MetaapiMarketDataApi::new(Default::default(), Default::default(), request_api);

        let volatility = metaapi
            .get_current_volatility(symbol, Timeframe::Hour)
            .unwrap();

        assert_eq!(volatility, 271);
    }

    #[test]
    #[allow(non_snake_case)]
    fn tune_candle__should_return_properly_tuned_candle() {
        let request_api = TestRequestApi {};

        let metaapi =
            MetaapiMarketDataApi::new(Default::default(), Default::default(), request_api);

        let candle_for_tuning = MetatraderCandleJson {
            time: "2022-06-21T13:00:00.000Z".to_string(),
            open: dec!(1.22664),
            high: dec!(1.22943),
            low: dec!(1.22655),
            close: dec!(1.22857),
            broker_time: "2022-06-21 16:00:00.000".to_string(),
        };

        let mut tuned_candle = metaapi.tune_candle(&candle_for_tuning, 271).unwrap();
        tuned_candle.size = tuned_candle.size.round();

        let expected_tuned_candle = BasicCandleProperties {
            time: from_naive_str_to_naive_datetime(&candle_for_tuning.broker_time).unwrap(),
            r#type: CandleType::Green,
            size: dec!(288),
            volatility: 271,
            prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22943),
                low: dec!(1.22655),
                close: dec!(1.22857),
            },
        };

        assert_eq!(tuned_candle, expected_tuned_candle);
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_all_volatilities__should_return_correct_values() {
        let request_api = TestRequestApi {};

        let metaapi =
            MetaapiMarketDataApi::new(Default::default(), Default::default(), request_api);

        let candles = vec![
            MetatraderCandleJson {
                time: "2022-06-21T10:00:00.000Z".to_string(),
                open: dec!(1.22958),
                high: dec!(1.23006),
                low: dec!(1.22781),
                close: dec!(1.22806),
                broker_time: "2022-06-21 13:00:00.000".to_string(),
            },
            MetatraderCandleJson {
                time: "2022-06-21T11:00:00.000Z".to_string(),
                open: dec!(1.22805),
                high: dec!(1.22863),
                low: dec!(1.22507),
                close: dec!(1.22685),
                broker_time: "2022-06-21 14:00:00.000".to_string(),
            },
            MetatraderCandleJson {
                time: "2022-06-21T12:00:00.000Z".to_string(),
                open: dec!(1.22686),
                high: dec!(1.22812),
                low: dec!(1.22596),
                close: dec!(1.22662),
                broker_time: "2022-06-21 15:00:00.000".to_string(),
            },
            MetatraderCandleJson {
                time: "2022-06-21T13:00:00.000Z".to_string(),
                open: dec!(1.22664),
                high: dec!(1.22943),
                low: dec!(1.22655),
                close: dec!(1.22857),
                broker_time: "2022-06-21 16:00:00.000".to_string(),
            },
        ];

        let volatilities = metaapi.get_all_volatilities(&candles, 2).unwrap();

        assert_eq!(volatilities, vec![None, Some(290), Some(286), Some(252)]);
    }
}
