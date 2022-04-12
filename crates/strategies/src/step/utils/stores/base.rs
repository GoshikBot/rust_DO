use std::collections::HashSet;

use anyhow::Result;

use base::entities::candle::BasicCandle;
use base::entities::order::BasicOrder;
use base::entities::{
    candle::CandleId, order::OrderId, tick::TickId, CandleBaseProperties, CandleEdgePrices,
    OrderBasePrices, OrderBaseProperties, TickBaseProperties,
};

use crate::step::utils::entities::working_levels::CorridorType;
use crate::step::utils::entities::{
    angles::{AngleBaseProperties, AngleId},
    working_levels::{WLId, WLMaxCrossingValue, WorkingLevelBaseProperties},
    Diff,
};

pub trait StepBaseStore {
    fn get_angle_base_properties_by_id(&self, id: &str) -> Result<Option<AngleBaseProperties>>;
    fn update_angle_base_properties(
        &mut self,
        id: &str,
        new_angle: AngleBaseProperties,
    ) -> Result<()>;
    fn get_all_angles(&self) -> Result<HashSet<AngleId>>;

    fn get_angle_of_second_level_after_bargaining_tendency_change(&self)
        -> Result<Option<AngleId>>;
    fn update_angle_of_second_level_after_bargaining_tendency_change(
        &mut self,
        new_angle: AngleId,
    ) -> Result<()>;

    fn get_tendency_change_angle(&self) -> Result<Option<AngleId>>;
    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_min_angle(&self) -> Result<Option<AngleId>>;
    fn update_min_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_virtual_min_angle(&self) -> Result<Option<AngleId>>;
    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_max_angle(&self) -> Result<Option<AngleId>>;
    fn update_max_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_virtual_max_angle(&self) -> Result<Option<AngleId>>;
    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_min_angle_before_bargaining_corridor(&self) -> Result<Option<AngleId>>;
    fn update_min_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_max_angle_before_bargaining_corridor(&self) -> Result<Option<AngleId>>;
    fn update_max_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_current_diff(&self) -> Result<Option<Diff>>;
    fn update_current_diff(&mut self, new_diff: Diff) -> Result<()>;

    fn get_previous_diff(&self) -> Result<Option<Diff>>;
    fn update_previous_diff(&mut self, new_diff: Diff) -> Result<()>;

    fn get_tick_base_properties_by_id(&self, tick_id: &str) -> Result<Option<TickBaseProperties>>;
    fn get_all_ticks(&self) -> Result<HashSet<TickId>>;

    fn update_candle_base_properties(
        &mut self,
        id: &str,
        new_base_properties: CandleBaseProperties,
    ) -> Result<()>;
    fn get_candle_base_properties_by_id(
        &self,
        candle_id: &str,
    ) -> Result<Option<CandleBaseProperties>>;

    fn get_candle_edge_prices_by_id(&self, id: &str) -> Result<Option<CandleEdgePrices>>;
    fn get_all_candles(&self) -> Result<HashSet<CandleId>>;

    fn get_current_tick(&self) -> Result<Option<TickId>>;
    fn update_current_tick(&mut self, tick_id: TickId) -> Result<()>;

    fn get_previous_tick(&self) -> Result<Option<TickId>>;
    fn update_previous_tick(&mut self, tick_id: TickId) -> Result<()>;

    fn get_current_candle(&self) -> Result<Option<CandleId>>;
    fn update_current_candle(&mut self, candle_id: CandleId) -> Result<()>;

    fn get_previous_candle(&self) -> Result<Option<CandleId>>;
    fn update_previous_candle(&mut self, candle_id: CandleId) -> Result<()>;

    fn remove_unused_items(&mut self) -> Result<()>;

    fn get_working_level_base_properties_by_id(
        &self,
        id: &str,
    ) -> Result<Option<WorkingLevelBaseProperties>>;
    fn update_working_level_base_properties(
        &mut self,
        id: &str,
        new_base_properties: WorkingLevelBaseProperties,
    ) -> Result<()>;

    fn move_working_level_to_active(&mut self, id: &str) -> Result<()>;
    fn move_working_level_to_removed(&mut self, id: &str) -> Result<()>;
    fn remove_working_level(&mut self, id: &str) -> Result<()>;

    fn get_created_working_levels(&self) -> Result<HashSet<WLId>>;
    fn get_active_working_levels(&self) -> Result<HashSet<WLId>>;
    fn get_removed_working_levels(&self) -> Result<HashSet<WLId>>;

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
    ) -> Result<Option<HashSet<CandleId>>>;

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

    fn get_order_base_prices_by_id(&self, id: &str) -> Result<Option<OrderBasePrices>>;
    fn get_order_base_properties_by_id(&self, id: &str) -> Result<Option<OrderBaseProperties>>;

    fn add_order_to_working_level_chain_of_orders(
        &mut self,
        working_level_id: &str,
        order_id: OrderId,
    ) -> Result<()>;
    fn get_working_level_chain_of_orders(
        &self,
        working_level_id: &str,
    ) -> Result<Option<HashSet<OrderId>>>;
}

pub trait StepBacktestingStore {
    fn create_angle(
        &mut self,
        id: AngleId,
        angle_base_properties: AngleBaseProperties,
    ) -> Result<()>;

    fn get_angle_by_id(&self, id: &str) -> Result<Option<AngleBaseProperties>>;

    fn create_tick(&mut self, id: TickId, tick_base_properties: TickBaseProperties) -> Result<()>;

    fn get_tick_by_id(&self, id: &str) -> Result<Option<TickBaseProperties>>;

    fn create_candle(
        &mut self,
        id: CandleId,
        base_properties: CandleBaseProperties,
        edge_prices: CandleEdgePrices,
    ) -> Result<()>;

    fn get_candle_by_id(&self, id: &str) -> Result<Option<BasicCandle>>;

    fn create_working_level(
        &mut self,
        id: WLId,
        base_properties: WorkingLevelBaseProperties,
    ) -> Result<()>;

    fn get_working_level_by_id(&self, id: &str) -> Result<Option<WorkingLevelBaseProperties>>;

    fn create_order(
        &mut self,
        id: OrderId,
        base_prices: OrderBasePrices,
        base_properties: OrderBaseProperties,
    ) -> Result<()>;

    fn get_order_by_id(&self, id: &str) -> Result<Option<BasicOrder>>;
}
