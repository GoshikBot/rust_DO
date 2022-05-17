use crate::metaapi_market_data_api::Timeframe;
use anyhow::Result;
use base::entities::candle::BasicCandle;
use base::entities::BasicTick;
use chrono::{DateTime, Duration, Utc};

pub trait MarketDataApi {
    fn get_current_tick(&self, symbol: &str) -> Result<BasicTick>;

    fn get_current_candle(&self, symbol: &str, timeframe: Timeframe) -> Result<BasicCandle>;

    fn get_historical_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<BasicCandle>>>;

    fn get_historical_ticks(
        &self,
        symbol: &str,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<Option<BasicTick>>>;
}
