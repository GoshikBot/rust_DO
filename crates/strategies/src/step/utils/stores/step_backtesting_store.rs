use std::collections::HashSet;

use anyhow::Result;

use base::entities::{
    candle::CandleId, tick::TickId, BasicTick, CandleBaseProperties, CandleEdgePrices, Level,
};

use crate::step::utils::entities::candle::Candle;
use crate::step::utils::entities::order::{Order, OrderId, OrderPrices, OrderProperties};
use crate::step::utils::entities::tick::Tick;
use crate::step::utils::entities::working_levels::CorridorType;
use crate::step::utils::entities::{
    angle::{Angle, AngleId},
    working_levels::{WLId, WLMaxCrossingValue, WorkingLevel},
    Diff,
};

pub trait StepBacktestingStore {
    fn create_angle(&mut self, id: AngleId, candle_id: CandleId, r#type: Level) -> Result<()>;
    fn get_angle_by_id(&self, id: &str) -> Result<Option<Angle>>;
    fn get_all_angles(&self) -> Result<HashSet<AngleId>>;

    fn get_angle_of_second_level_after_bargaining_tendency_change(&self) -> Result<Option<Angle>>;
    fn update_angle_of_second_level_after_bargaining_tendency_change(
        &mut self,
        new_angle: AngleId,
    ) -> Result<()>;

    fn get_tendency_change_angle(&self) -> Result<Option<Angle>>;
    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_min_angle(&self) -> Result<Option<Angle>>;
    fn update_min_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_virtual_min_angle(&self) -> Result<Option<Angle>>;
    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_max_angle(&self) -> Result<Option<Angle>>;
    fn update_max_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_virtual_max_angle(&self) -> Result<Option<Angle>>;
    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_min_angle_before_bargaining_corridor(&self) -> Result<Option<Angle>>;
    fn update_min_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()>;

    fn get_max_angle_before_bargaining_corridor(&self) -> Result<Option<Angle>>;
    fn update_max_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()>;

    fn create_tick(&mut self, id: TickId, tick_base_properties: BasicTick) -> Result<()>;
    fn get_tick_by_id(&self, tick_id: &str) -> Result<Option<Tick>>;
    fn get_all_ticks(&self) -> Result<HashSet<TickId>>;

    fn create_candle(
        &mut self,
        id: CandleId,
        base_properties: CandleBaseProperties,
        edge_prices: CandleEdgePrices,
    ) -> Result<()>;
    fn get_candle_by_id(&self, candle_id: &str) -> Result<Option<Candle>>;

    fn get_all_candles(&self) -> Result<HashSet<CandleId>>;

    fn get_current_tick(&self) -> Result<Option<Tick>>;
    fn update_current_tick(&mut self, tick_id: TickId) -> Result<()>;

    fn get_previous_tick(&self) -> Result<Option<Tick>>;
    fn update_previous_tick(&mut self, tick_id: TickId) -> Result<()>;

    fn get_current_candle(&self) -> Result<Option<Candle>>;
    fn update_current_candle(&mut self, candle_id: CandleId) -> Result<()>;

    fn get_previous_candle(&self) -> Result<Option<Candle>>;
    fn update_previous_candle(&mut self, candle_id: CandleId) -> Result<()>;

    fn remove_unused_items(&mut self) -> Result<()>;

    fn create_working_level(&mut self, id: WLId, base_properties: WorkingLevel) -> Result<()>;
    fn get_working_level_by_id(&self, id: &str) -> Result<Option<WorkingLevel>>;

    fn move_working_level_to_active(&mut self, id: &str) -> Result<()>;
    fn move_working_level_to_removed(&mut self, id: &str) -> Result<()>;
    fn remove_working_level(&mut self, id: &str) -> Result<()>;

    fn get_created_working_levels(&self) -> Result<Vec<WorkingLevel>>;
    fn get_active_working_levels(&self) -> Result<Vec<WorkingLevel>>;
    fn get_removed_working_levels(&self) -> Result<Vec<WorkingLevel>>;

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
    ) -> Result<Option<Vec<Candle>>>;

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

    fn create_order(
        &mut self,
        id: OrderId,
        base_prices: OrderPrices,
        base_properties: OrderProperties,
    ) -> Result<()>;
    fn get_order_by_id(&self, id: &str) -> Result<Option<Order>>;

    fn add_order_to_working_level_chain_of_orders(
        &mut self,
        working_level_id: &str,
        order_id: OrderId,
    ) -> Result<()>;
    fn get_working_level_chain_of_orders(
        &self,
        working_level_id: &str,
    ) -> Result<Option<Vec<Order>>>;
}
