use anyhow::Result;
use base::entities::candle::CandleId;
use base::entities::Item;
use base::stores::candle_store::BasicCandleStore;

pub trait StepCandleStore: BasicCandleStore {
    fn get_candles_of_general_corridor(
        &self,
    ) -> Result<Vec<Item<CandleId, Self::CandleProperties>>>;

    fn add_candle_to_general_corridor(&mut self, candle_id: CandleId) -> Result<()>;

    fn clear_general_corridor(&mut self) -> Result<()>;
}
