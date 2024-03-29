use anyhow::Result;
use base::entities::order::OrderId;
use base::entities::{candle::CandleId, Item};
use base::params::ParamOutputValue;

use crate::step::utils::entities::working_levels::{
    CorridorType, WLId, WLMaxCrossingValue, WLStatus,
};

pub trait StepWorkingLevelStore {
    type WorkingLevelProperties;
    type CandleProperties;
    type OrderProperties;

    fn create_working_level(
        &mut self,
        id: WLId,
        properties: Self::WorkingLevelProperties,
    ) -> Result<Item<WLId, Self::WorkingLevelProperties>>;
    fn get_working_level_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Item<WLId, Self::WorkingLevelProperties>>>;

    fn move_working_level_to_active(&mut self, id: &str) -> Result<()>;
    fn remove_working_level(&mut self, id: &str) -> Result<()>;

    fn get_created_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>>;
    fn get_active_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>>;
    fn get_all_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>>;

    fn get_working_level_status(&self, id: &str) -> Result<Option<WLStatus>>;

    fn clear_working_level_corridor(
        &mut self,
        working_level_id: &str,
        corridor_type: CorridorType,
    ) -> Result<()>;

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

    fn move_take_profits_of_level(
        &mut self,
        working_level_id: &str,
        distance_to_move_take_profits: ParamOutputValue,
    ) -> Result<()>;

    fn take_profits_of_level_are_moved(&self, working_level_id: &str) -> Result<bool>;

    fn get_working_level_chain_of_orders(
        &self,
        working_level_id: &str,
    ) -> Result<Vec<Item<OrderId, Self::OrderProperties>>>;
}
