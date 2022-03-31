use backtesting::{Balance, Units, Trades, Leverage, Spread, backtesting_base_store::BacktestingBaseStore};
use base::entities::{MovementType, TickBaseProperties, CandleBaseProperties, CandleEdgePrices, candle::{CandleId, CandleSize, CandleVolatility}, Level, tick::{TickId, TickPrice}, CandleType, OrderBasePrices, OrderBaseProperties, order::OrderId};
use chrono::NaiveDateTime;
use simple_error::SimpleResult;

use crate::step::utils::entities::{strategies::{Symbol, BacktestingStatisticNumber}, angles::{Angle, AngleId}, Diff, settings::{SettingProgramName, RatioSettingValue, PointSettingValue}, working_levels::{WorkingLevelBaseProperties, WLId, WLMaxCrossingValue, WLIndex}};

pub trait StepBaseStore {
    fn get_symbol(&self) -> SimpleResult<Symbol>;

    fn get_tendency(&self) -> SimpleResult<MovementType>;
    fn update_tendency(&mut self, value: MovementType) -> SimpleResult<()>;

    fn get_tendency_changed_on_crossing_bargaining_corridor(&self) -> SimpleResult<bool>;
    fn update_tendency_changed_on_crossing_bargaining_corridor(&mut self, value: bool) -> SimpleResult<()>;

    fn get_second_level_after_bargaining_tendency_change_is_created(&self) -> SimpleResult<bool>;
    fn update_second_level_after_bargaining_tendency_change_is_created(&mut self, value: bool) -> SimpleResult<()>;

    fn get_skip_creating_new_working_level(&self) -> SimpleResult<bool>;
    fn update_skip_creating_new_working_level(&mut self, value: bool) -> SimpleResult<()>;

    fn get_no_trading_mode(&self) -> SimpleResult<bool>;
    fn update_no_trading_mode(&mut self, value: bool) -> SimpleResult<()>;

    fn create_angle(&mut self, id: AngleId, new_angle: Angle) -> SimpleResult<()>; 
    fn get_angle_by_id(&self, id: AngleId) -> SimpleResult<Option<Angle>>;
    fn update_angle(&self, id: AngleId, new_angle: Angle) -> SimpleResult<()>;

    fn get_angle_of_second_level_after_bargaining_tendency_change(&self) -> SimpleResult<Option<AngleId>>;
    fn update_angle_of_second_level_after_bargaining_tendency_change(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_tendency_change_angle(&self) -> SimpleResult<Option<AngleId>>;
    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_min_angle(&self) -> SimpleResult<Option<AngleId>>;
    fn update_min_angle(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_virtual_min_angle(&self) -> SimpleResult<Option<AngleId>>;
    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_max_angle(&self) -> SimpleResult<Option<AngleId>>;
    fn update_max_angle(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_virtual_max_angle(&self) -> SimpleResult<Option<AngleId>>;
    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_min_angle_before_bargaining_corridor(&self) -> SimpleResult<Option<AngleId>>;
    fn update_min_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_max_angle_before_bargaining_corridor(&self) -> SimpleResult<Option<AngleId>>;
    fn update_max_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> SimpleResult<()>;

    fn get_current_diff(&self) -> SimpleResult<Option<Diff>>;
    fn update_current_diff(&mut self, new_diff: Diff) -> SimpleResult<()>;

    fn get_previous_diff(&self) -> SimpleResult<Option<Diff>>;
    fn update_previous_diff(&mut self, new_diff: Diff) -> SimpleResult<()>;

    fn create_tick(&mut self, id: TickId, base_properties: TickBaseProperties) -> SimpleResult<()>;
    fn get_tick_base_properties_by_id(&self, tick_id: TickId) -> SimpleResult<Option<TickBaseProperties>>;
    fn remove_unused_ticks(&mut self) -> SimpleResult<()>;

    fn create_candle(&mut self, id: CandleId, base_properties: CandleBaseProperties, edge_prices: CandleEdgePrices) -> SimpleResult<()>;
    fn update_candle_base_properties(&mut self, id: CandleId, new_base_properties: CandleBaseProperties) -> SimpleResult<()>;
    fn get_candle_base_properties_by_id(&self, candle_id: CandleId) -> SimpleResult<Option<CandleBaseProperties>>;
    fn get_candle_edge_prices_by_id(&self, candle_id: CandleId) -> SimpleResult<Option<CandleEdgePrices>>;
    fn remove_unused_candles(&mut self) -> SimpleResult<()>;

    fn get_current_tick(&self) -> SimpleResult<Option<TickId>>;
    fn get_previous_tick(&self) -> SimpleResult<Option<TickId>>;

    fn get_current_candle(&self) -> SimpleResult<Option<CandleId>>;
    fn get_previous_candle(&self) -> SimpleResult<Option<CandleId>>;

    fn add_ratio_setting(&mut self, name: SettingProgramName, value: RatioSettingValue) -> SimpleResult<()>;

    fn add_point_setting(&mut self, name: SettingProgramName, value: PointSettingValue) -> SimpleResult<()>;

    fn get_ratio_setting(&self, name: SettingProgramName) -> SimpleResult<Option<RatioSettingValue>>;

    fn get_point_setting(&self, name: SettingProgramName) -> SimpleResult<Option<PointSettingValue>>;

    fn create_working_level(&mut self, id: WLId, base_properties: WorkingLevelBaseProperties) -> SimpleResult<()>;
    fn get_working_level_base_properties_by_id(&self, id: WLId) -> SimpleResult<Option<WorkingLevelBaseProperties>>;
    fn update_working_level_base_properties(&mut self, id: WLId, new_base_properties: WorkingLevelBaseProperties) -> SimpleResult<()>;

    fn move_working_level_to_active(&mut self, id: WLId) -> SimpleResult<()>;
    fn move_working_level_to_removed(&mut self, id: WLId) -> SimpleResult<()>;
    fn remove_working_level(&mut self, id: WLId) -> SimpleResult<()>;

    fn get_created_working_levels(&self) -> SimpleResult<Vec<WLId>>;
    fn get_active_working_levels(&self) -> SimpleResult<Vec<WLId>>;
    fn get_removed_working_levels(&self) -> SimpleResult<Vec<WLId>>;

    fn add_candle_to_working_level_small_corridor(&mut self, working_level_id: WLId, candle_id: CandleId) -> SimpleResult<()>;
    fn get_candles_of_working_level_small_corridor(&self, working_level_id: WLId) -> SimpleResult<Vec<CandleId>>;

    fn add_candle_to_working_level_big_corridor(&mut self, working_level_id: WLId, candle_id: CandleId) -> SimpleResult<()>;
    fn get_candles_of_working_level_big_corridor(&self, working_level_id: WLId) -> SimpleResult<Vec<CandleId>>;

    fn add_max_crossing_value_to_working_level(&mut self, working_level_id: WLId, value: WLMaxCrossingValue) -> SimpleResult<()>;
    fn update_max_crossing_value_of_working_level(&mut self, working_level_id: WLId, new_value: WLMaxCrossingValue) -> SimpleResult<()>;

    fn add_working_level_to_list_of_levels_with_moved_take_profits(&mut self, working_level_id: WLId) -> SimpleResult<()>;
    fn get_list_of_working_levels_with_moved_take_profits(&self) -> SimpleResult<Vec<WLId>>;

    fn create_order(&mut self, id: OrderId, base_prices: OrderBasePrices, base_properties: OrderBaseProperties) -> SimpleResult<()>;
    fn get_order_base_prices_by_id(&self, id: OrderId) -> SimpleResult<Option<OrderBasePrices>>;
    fn get_order_base_properties_by_id(&self, id: OrderId) -> SimpleResult<Option<OrderBaseProperties>>;
    fn remove_order(&mut self, id: OrderId) -> SimpleResult<()>;

    fn add_order_to_working_level_chain_of_orders(&mut self, working_level_id: WLId, order_id: OrderId) -> SimpleResult<()>;
    fn get_orders_of_working_level(&self, working_level_id: WLId) -> SimpleResult<Vec<OrderId>>;
}

pub trait StepBacktestingStore: BacktestingBaseStore + StepBaseStore {
    fn get_number_of_working_levels(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_number_of_working_levels(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_number_of_tendency_changes(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_number_of_tendency_changes(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_deleted_by_being_close_to_another_one(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_being_close_to_another_one(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_deleted_by_another_active_chain_of_orders(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_another_active_chain_of_orders(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_deleted_by_expiration_by_distance(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_expiration_by_distance(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_deleted_by_expiration_by_time(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_expiration_by_time(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_deleted_by_price_being_beyond_stop_loss(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_price_being_beyond_stop_loss(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;
    
    fn get_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn get_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;
    
    fn get_deleted_by_exceeding_activation_crossing_distance(&self) -> SimpleResult<BacktestingStatisticNumber>;
    fn update_deleted_by_exceeding_activation_crossing_distance(&mut self, new_number: BacktestingStatisticNumber) -> SimpleResult<()>;

    fn assign_index_to_working_level(&mut self, working_level_id: WLId, index: WLIndex) -> SimpleResult<()>;
    fn get_index_of_working_level(&self, working_level_id: WLId) -> SimpleResult<WLIndex>;
}