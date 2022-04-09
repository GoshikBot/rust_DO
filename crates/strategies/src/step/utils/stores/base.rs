use backtesting::{
    backtesting_base_store::BacktestingBaseStore, Balance, Leverage, Spread, Trades, Units,
};
use base::entities::{
    candle::{CandleId, CandleSize, CandleVolatility},
    order::OrderId,
    tick::{TickId, TickPrice},
    CandleBaseProperties, CandleEdgePrices, CandleType, Level, MovementType, OrderBasePrices,
    OrderBaseProperties, TickBaseProperties,
};
use chrono::NaiveDateTime;
use std::collections::HashSet;

use crate::step::utils::entities::working_levels::CorridorType;
use crate::step::utils::entities::{
    angles::{Angle, AngleId},
    strategies::{BacktestingStatisticNumber, Symbol},
    working_levels::{WLId, WLIndex, WLMaxCrossingValue, WorkingLevelBaseProperties},
    Diff,
};

use anyhow::Result;

pub trait StepBaseStore {
    fn get_symbol(&self) -> Result<Symbol>;

    fn get_tendency(&self) -> Result<MovementType>;
    fn update_tendency(&mut self, value: MovementType) -> Result<()>;

    fn get_tendency_changed_on_crossing_bargaining_corridor(&self) -> Result<bool>;

    fn update_tendency_changed_on_crossing_bargaining_corridor(
        &mut self,
        value: bool,
    ) -> Result<()>;

    fn get_second_level_after_bargaining_tendency_change_is_created(&self) -> Result<bool>;
    fn update_second_level_after_bargaining_tendency_change_is_created(
        &mut self,
        value: bool,
    ) -> Result<()>;

    fn get_skip_creating_new_working_level(&self) -> Result<bool>;
    fn update_skip_creating_new_working_level(&mut self, value: bool) -> Result<()>;

    fn get_no_trading_mode(&self) -> Result<bool>;
    fn update_no_trading_mode(&mut self, value: bool) -> Result<()>;

    fn create_angle(&mut self, id: AngleId, new_angle: Angle) -> Result<()>;
    fn get_angle_by_id(&self, id: &str) -> Result<Option<Angle>>;
    fn update_angle(&mut self, id: &str, new_angle: Angle) -> Result<()>;
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

    fn create_tick(&mut self, id: TickId, base_properties: TickBaseProperties) -> Result<()>;
    fn get_tick_base_properties_by_id(&self, tick_id: &str) -> Result<Option<TickBaseProperties>>;
    fn get_all_ticks(&self) -> Result<HashSet<TickId>>;

    fn create_candle(
        &mut self,
        id: CandleId,
        base_properties: CandleBaseProperties,
        edge_prices: CandleEdgePrices,
    ) -> Result<()>;
    fn update_candle_base_properties(
        &mut self,
        id: &str,
        new_base_properties: CandleBaseProperties,
    ) -> Result<()>;
    fn get_candle_base_properties_by_id(
        &self,
        candle_id: &str,
    ) -> Result<Option<CandleBaseProperties>>;
    fn get_candle_edge_prices_by_id(
        &self,
        candle_id: &CandleId,
    ) -> Result<Option<CandleEdgePrices>>;
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

    fn create_working_level(
        &mut self,
        id: WLId,
        base_properties: WorkingLevelBaseProperties,
    ) -> Result<()>;
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

    fn create_order(
        &mut self,
        id: OrderId,
        base_prices: OrderBasePrices,
        base_properties: OrderBaseProperties,
    ) -> Result<()>;
    fn get_order_base_prices_by_id(&self, id: &str) -> Result<Option<OrderBasePrices>>;
    fn get_order_base_properties_by_id(&self, id: &str) -> Result<Option<OrderBaseProperties>>;
    fn remove_order(&mut self, id: &str) -> Result<()>;

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

pub trait StepBacktestingStore: BacktestingBaseStore + StepBaseStore {
    fn get_number_of_working_levels(&self) -> Result<BacktestingStatisticNumber>;
    fn update_number_of_working_levels(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_number_of_tendency_changes(&self) -> Result<BacktestingStatisticNumber>;
    fn update_number_of_tendency_changes(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_being_close_to_another_one(&self) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_being_close_to_another_one(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_another_active_chain_of_orders(&self) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_another_active_chain_of_orders(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_expiration_by_distance(&self) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_expiration_by_distance(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_expiration_by_time(&self) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_expiration_by_time(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_price_being_beyond_stop_loss(&self) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_price_being_beyond_stop_loss(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(
        &self,
    ) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(
        &self,
    ) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;

    fn get_deleted_by_exceeding_activation_crossing_distance(
        &self,
    ) -> Result<BacktestingStatisticNumber>;
    fn update_deleted_by_exceeding_activation_crossing_distance(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()>;
}
