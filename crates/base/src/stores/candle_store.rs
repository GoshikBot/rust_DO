use crate::entities::candle::CandleId;
use crate::entities::Item;
use anyhow::Result;

pub trait BasicCandleStore {
    type CandleProperties;

    fn create_candle(&mut self, properties: Self::CandleProperties) -> Result<CandleId>;
    fn get_candle_by_id(
        &self,
        candle_id: &str,
    ) -> Result<Option<Item<CandleId, Self::CandleProperties>>>;

    fn get_current_candle(&self) -> Result<Option<Item<CandleId, Self::CandleProperties>>>;
    fn update_current_candle(&mut self, candle_id: CandleId) -> Result<()>;

    fn get_previous_candle(&self) -> Result<Option<Item<CandleId, Self::CandleProperties>>>;
    fn update_previous_candle(&mut self, candle_id: CandleId) -> Result<()>;
}
