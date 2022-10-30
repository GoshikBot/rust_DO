use anyhow::Result;
use base::entities::candle::BasicCandleProperties;
use base::entities::{BasicTickProperties, Timeframe};
use chrono::{DateTime, Duration, Utc};

pub mod helpers;
pub mod metaapi_market_data_api;

pub use crate::metaapi_market_data_api::{MetaapiMarketDataApi, RetrySettings};

pub trait MarketDataApi {
    type RealTickProperties;
    type HistoricalTickProperties;
    type CandleProperties;

    fn get_current_tick(&self, symbol: &str) -> Result<Self::RealTickProperties>;

    fn get_current_candle(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<Self::CandleProperties>;

    fn get_historical_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<Self::CandleProperties>>>;

    fn get_historical_ticks(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<Self::HistoricalTickProperties>>>;
}
