use crate::entities::HistoricalTimeframe;
use anyhow::Result;
use base::entities::candle::BasicCandle;
use base::entities::TickBaseProperties;
use chrono::{DateTime, Duration, Utc};

pub trait MarketDataApi {
    fn get_current_tick(&self, symbol: &str) -> Result<TickBaseProperties>;
    fn get_current_candle(&self, symbol: &str, timeframe: &str) -> Result<BasicCandle>;
    fn get_historical_candles(
        &self,
        symbol: &str,
        timeframe: HistoricalTimeframe,
        end_time: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Vec<BasicCandle>>;
}
