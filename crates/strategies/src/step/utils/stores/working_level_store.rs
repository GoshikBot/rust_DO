use anyhow::Result;
use base::entities::order::OrderId;
use base::entities::{candle::CandleId, Item};

use crate::step::utils::entities::working_levels::{CorridorType, WLId, WLMaxCrossingValue};

pub trait StepWorkingLevelStore {
    type WorkingLevelProperties;
    type CandleProperties;
    type OrderProperties;

    fn create_working_level(&mut self, properties: Self::WorkingLevelProperties) -> Result<WLId>;
    fn get_working_level_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Item<WLId, Self::WorkingLevelProperties>>>;

    fn move_working_level_to_active(&mut self, id: &str) -> Result<()>;
    fn move_working_level_to_removed(&mut self, id: &str) -> Result<()>;
    fn remove_working_level(&mut self, id: &str) -> Result<()>;

    fn get_created_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>>;
    fn get_active_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>>;
    fn get_removed_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>>;

    fn add_candle_to_working_level_corridor(
        &mut self,
        working_level_id: &str,
        candle_id: CandleId,
        corridor_type: CorridorType,
    ) -> Result<()>;

    fn get_candles_of_working_level_corridor(
        &self,
        working_level_id: &str,
        corridor_type: CorridorType,
    ) -> Result<Vec<Item<CandleId, Self::CandleProperties>>>;

    fn update_max_crossing_value_of_working_level(
        &mut self,
        working_level_id: &str,
        new_value: WLMaxCrossingValue,
    ) -> Result<()>;

    fn get_max_crossing_value_of_working_level(
        &self,
        working_level_id: &str,
    ) -> Result<Option<WLMaxCrossingValue>>;

    fn move_take_profits_of_level(&mut self, working_level_id: &str) -> Result<()>;

    fn are_take_profits_of_level_moved(&self, working_level_id: &str) -> Result<bool>;

    fn add_order_to_working_level_chain_of_orders(
        &mut self,
        working_level_id: &str,
        order_id: OrderId,
    ) -> Result<()>;

    fn get_working_level_chain_of_orders(
        &self,
        working_level_id: &str,
    ) -> Result<Vec<Item<OrderId, Self::OrderProperties>>>;
}
