use anyhow::Result;
use base::entities::candle::BasicCandle;
use base::entities::TickBaseProperties;

pub trait TradingAPI {
    fn get_current_tick(&self, symbol: &str) -> Result<TickBaseProperties>;
    fn get_current_candle(&self, symbol: &str, timeframe: &str) -> Result<BasicCandle>;
}
