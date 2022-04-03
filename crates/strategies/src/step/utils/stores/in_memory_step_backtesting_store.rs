use std::collections::{HashMap, HashSet};

use backtesting::{
    backtesting_base_store::BacktestingBaseStore, BacktestingConfig, BacktestingLowLevelData,
    Balance, Leverage, Spread, Trades, Units,
};
use base::entities::{
    candle::{CandleEdgePrice, CandleId},
    order::OrderId,
    tick::TickId,
    CandleBaseProperties, CandleEdgePrices, Level, MovementType, OrderBaseProperties,
    TickBaseProperties,
};
use simple_error::{SimpleError, SimpleResult};

use crate::step::utils::entities::working_levels::CorridorType;
use crate::step::utils::entities::{
    angles::{Angle, AngleId},
    settings::{PointSettingValue, RatioSettingValue, SettingProgramName, SettingTableName},
    strategies::{
        BacktestingIndex, BacktestingIndexes, BacktestingStatisticNumber, BacktestingStatistics,
        StrategyAngles, StrategyBaseConfig, StrategyDiffs, StrategyTicksCandles, Symbol,
    },
    working_levels::{
        WLId, WorkingLevelBaseProperties, WorkingLevelCorridorCandle, WorkingLevelMaxCrossingValue,
        WorkingLevelOrder,
    },
    Diff,
};

use super::base::{StepBacktestingStore, StepBaseStore};

#[derive(Default)]
pub struct InMemoryStepBacktestingStore {
    candle_base_properties: HashMap<CandleId, CandleBaseProperties>,
    candle_edge_prices: HashMap<CandleId, CandleEdgePrices>,

    tick_base_properties: HashMap<TickId, TickBaseProperties>,

    angles: HashMap<AngleId, Angle>,

    working_level_base_properties: HashMap<WLId, WorkingLevelBaseProperties>,
    working_level_max_crossing_values: HashMap<WLId, WorkingLevelMaxCrossingValue>,
    working_level_small_corridors: Vec<WorkingLevelCorridorCandle>,
    working_level_big_corridors: Vec<WorkingLevelCorridorCandle>,

    working_level_chain_of_orders: Vec<WorkingLevelOrder>,

    order_base_prices: HashMap<OrderId, OrderBaseProperties>,
    order_base_properties: HashMap<OrderId, OrderBaseProperties>,

    backtesting_limit_orders: Vec<OrderId>,

    setting_names: HashMap<SettingProgramName, SettingTableName>,
    ratio_settings: HashMap<SettingProgramName, RatioSettingValue>,
    point_settings: HashMap<SettingProgramName, PointSettingValue>,

    strategy_base_config: StrategyBaseConfig,
    strategy_angles: StrategyAngles,
    strategy_diffs: StrategyDiffs,
    strategy_ticks_candles: StrategyTicksCandles,

    backtesting_indexes: BacktestingIndexes,
    backtesting_statistics: BacktestingStatistics,

    backtesting_low_level_data: BacktestingLowLevelData,
    backtesting_config: BacktestingConfig,
}

impl InMemoryStepBacktestingStore {
    pub fn new() -> Self {
        todo!()
    }

    /// For each tick checks whether it is in use. If a tick is not in use,
    /// removes it. We don't want tick list to grow endlessly.
    fn remove_unused_ticks(&mut self) {
        let tick_is_in_use = |tick_id: &TickId| {
            [
                self.strategy_ticks_candles.previous_tick.as_ref(),
                self.strategy_ticks_candles.current_tick.as_ref(),
            ]
            .contains(&Some(tick_id))
        };

        self.tick_base_properties
            .retain(|tick_id, _| tick_is_in_use(tick_id));
    }

    /// For each candle checks whether it is in use. If a candle is not in use,
    /// removes it. We don't want candle list to grow endlessly.
    fn remove_unused_candles(&mut self) {
        let mut candles_to_remove = HashSet::new();

        let candle_in_angles = |candle_id: &CandleId| self.angles.contains_key(candle_id);
        let candle_in_corridors = |candle_id: &CandleId| {
            self.working_level_small_corridors
                .iter()
                .any(|level_candle| &level_candle.candle_id == candle_id)
                || self
                    .working_level_big_corridors
                    .iter()
                    .any(|level_candle| &level_candle.candle_id == candle_id)
        };
        let is_current_candle = |candle_id: &CandleId| {
            self.strategy_ticks_candles.current_candle.as_ref() == Some(candle_id)
        };
        let is_previous_candle = |candle_id: &CandleId| {
            self.strategy_ticks_candles.previous_candle.as_ref() == Some(candle_id)
        };

        self.candle_base_properties.retain(|candle_id, _| {
            if candle_in_angles(candle_id)
                || candle_in_corridors(candle_id)
                || is_current_candle(candle_id)
                || is_previous_candle(candle_id)
            {
                return true;
            }

            candles_to_remove.insert(candle_id.clone());
            false
        });

        self.candle_edge_prices
            .retain(|candle_id, _| candles_to_remove.contains(candle_id));
    }

    /// For each angle checks whether it is in use. If an angle is not in use,
    /// removes it. We don't want angle list to grow endlessly.
    fn remove_unused_angles(&mut self) {
        self.angles.retain(|angle_id, _| {
            [
                self.strategy_angles.max_angle.as_ref(),
                self.strategy_angles.min_angle.as_ref(),
                self.strategy_angles.virutal_max_angle.as_ref(),
                self.strategy_angles.virtual_min_angle.as_ref(),
                self.strategy_angles.tendency_change_angle.as_ref(),
                self.strategy_angles
                    .max_angle_before_bargaining_corridor
                    .as_ref(),
                self.strategy_angles
                    .angle_of_second_level_after_bargaining_tendency_change
                    .as_ref(),
                self.strategy_angles
                    .min_angle_before_bargaining_corridor
                    .as_ref(),
            ]
            .contains(&Some(angle_id))
        });
    }
}

impl StepBacktestingStore for InMemoryStepBacktestingStore {
    fn get_number_of_working_levels(&self) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_number_of_working_levels(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_number_of_tendency_changes(&self) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_number_of_tendency_changes(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_being_close_to_another_one(
        &self,
    ) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_being_close_to_another_one(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_another_active_chain_of_orders(
        &self,
    ) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_another_active_chain_of_orders(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_expiration_by_distance(&self) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_expiration_by_distance(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_expiration_by_time(&self) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_expiration_by_time(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_price_being_beyond_stop_loss(
        &self,
    ) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_price_being_beyond_stop_loss(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(
        &self,
    ) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(
        &self,
    ) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_deleted_by_exceeding_activation_crossing_distance(
        &self,
    ) -> SimpleResult<BacktestingStatisticNumber> {
        todo!()
    }

    fn update_deleted_by_exceeding_activation_crossing_distance(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn assign_index_to_working_level(
        &mut self,
        working_level_id: WLId,
        index: crate::step::utils::entities::working_levels::WLIndex,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_index_of_working_level(
        &self,
        working_level_id: WLId,
    ) -> SimpleResult<crate::step::utils::entities::working_levels::WLIndex> {
        todo!()
    }
}

impl BacktestingBaseStore for InMemoryStepBacktestingStore {
    fn get_initial_balance(&self) -> SimpleResult<Balance> {
        todo!()
    }

    fn get_processing_balance(&self) -> SimpleResult<Balance> {
        todo!()
    }

    fn update_processing_balance(&mut self, new_processing_balance: Balance) -> SimpleResult<()> {
        todo!()
    }

    fn get_real_balance(&self) -> SimpleResult<Balance> {
        todo!()
    }

    fn update_real_balance(&mut self, new_real_balance: Balance) -> SimpleResult<()> {
        todo!()
    }

    fn get_units(&self) -> SimpleResult<Units> {
        todo!()
    }

    fn update_units(&mut self, new_units: Units) -> SimpleResult<()> {
        todo!()
    }

    fn get_trades(&self) -> SimpleResult<Trades> {
        todo!()
    }

    fn update_trades(&mut self, new_trades: Trades) -> SimpleResult<()> {
        todo!()
    }

    fn get_leverage(&self) -> SimpleResult<Leverage> {
        todo!()
    }

    fn get_use_spread(&self) -> SimpleResult<bool> {
        todo!()
    }

    fn get_spread(&self) -> SimpleResult<Spread> {
        todo!()
    }

    fn add_limit_order(&mut self, order_id: OrderId) -> SimpleResult<()> {
        todo!()
    }

    fn remove_limit_order(&mut self, order_id: OrderId) -> SimpleResult<()> {
        todo!()
    }
}

impl StepBaseStore for InMemoryStepBacktestingStore {
    fn get_symbol(&self) -> SimpleResult<Symbol> {
        Ok(self.strategy_base_config.symbol)
    }

    fn get_tendency(&self) -> SimpleResult<MovementType> {
        Ok(self.strategy_base_config.tendency)
    }

    fn update_tendency(&mut self, value: MovementType) -> SimpleResult<()> {
        self.strategy_base_config.tendency = value;
        Ok(())
    }

    fn get_tendency_changed_on_crossing_bargaining_corridor(&self) -> SimpleResult<bool> {
        Ok(self
            .strategy_base_config
            .tendency_changed_on_crossing_bargaining_corridor)
    }

    fn update_tendency_changed_on_crossing_bargaining_corridor(
        &mut self,
        value: bool,
    ) -> SimpleResult<()> {
        self.strategy_base_config
            .tendency_changed_on_crossing_bargaining_corridor = value;
        Ok(())
    }

    fn get_second_level_after_bargaining_tendency_change_is_created(&self) -> SimpleResult<bool> {
        Ok(self
            .strategy_base_config
            .second_level_after_bargaining_tendency_change_is_created)
    }

    fn update_second_level_after_bargaining_tendency_change_is_created(
        &mut self,
        value: bool,
    ) -> SimpleResult<()> {
        self.strategy_base_config
            .second_level_after_bargaining_tendency_change_is_created = value;
        Ok(())
    }

    fn get_skip_creating_new_working_level(&self) -> SimpleResult<bool> {
        Ok(self.strategy_base_config.skip_creating_new_working_level)
    }

    fn update_skip_creating_new_working_level(&mut self, value: bool) -> SimpleResult<()> {
        self.strategy_base_config.skip_creating_new_working_level = value;
        Ok(())
    }

    fn get_no_trading_mode(&self) -> SimpleResult<bool> {
        Ok(self.strategy_base_config.no_trading_mode)
    }

    fn update_no_trading_mode(&mut self, value: bool) -> SimpleResult<()> {
        self.strategy_base_config.no_trading_mode = value;
        Ok(())
    }

    fn create_angle(&mut self, id: AngleId, new_angle: Angle) -> SimpleResult<()> {
        if let Some(angle) = self.angles.get(&id) {
            return Err(SimpleError::new(format!(
                "an angle with an id {} already exists: {:?}",
                id, angle
            )));
        }

        self.angles.insert(id, new_angle);

        Ok(())
    }

    fn get_angle_by_id(&self, id: &AngleId) -> SimpleResult<Option<Angle>> {
        Ok(self.angles.get(id).cloned())
    }

    fn update_angle(&mut self, id: AngleId, new_angle: Angle) -> SimpleResult<()> {
        if self.angles.get(&id).is_none() {
            return Err(SimpleError::new(format!(
                "there is no angle with an id {}",
                id
            )));
        }

        self.angles.insert(id, new_angle);

        Ok(())
    }

    fn get_all_angles(&self) -> SimpleResult<HashSet<AngleId>> {
        Ok(self.angles.keys().cloned().collect())
    }

    fn get_angle_of_second_level_after_bargaining_tendency_change(
        &self,
    ) -> SimpleResult<Option<AngleId>> {
        Ok(self
            .strategy_angles
            .angle_of_second_level_after_bargaining_tendency_change
            .clone())
    }

    fn update_angle_of_second_level_after_bargaining_tendency_change(
        &mut self,
        new_angle: AngleId,
    ) -> SimpleResult<()> {
        self.strategy_angles
            .angle_of_second_level_after_bargaining_tendency_change = Some(new_angle);
        Ok(())
    }

    fn get_tendency_change_angle(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self.strategy_angles.tendency_change_angle.clone())
    }

    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> SimpleResult<()> {
        self.strategy_angles.tendency_change_angle = Some(new_angle);
        Ok(())
    }

    fn get_min_angle(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self.strategy_angles.min_angle.clone())
    }

    fn update_min_angle(&mut self, new_angle: AngleId) -> SimpleResult<()> {
        self.strategy_angles.min_angle = Some(new_angle);
        Ok(())
    }

    fn get_virtual_min_angle(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self.strategy_angles.virtual_min_angle.clone())
    }

    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> SimpleResult<()> {
        self.strategy_angles.virtual_min_angle = Some(new_angle);
        Ok(())
    }

    fn get_max_angle(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self.strategy_angles.max_angle.clone())
    }

    fn update_max_angle(&mut self, new_angle: AngleId) -> SimpleResult<()> {
        self.strategy_angles.max_angle = Some(new_angle);
        Ok(())
    }

    fn get_virtual_max_angle(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self.strategy_angles.virutal_max_angle.clone())
    }

    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> SimpleResult<()> {
        self.strategy_angles.virutal_max_angle = Some(new_angle);
        Ok(())
    }

    fn get_min_angle_before_bargaining_corridor(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self
            .strategy_angles
            .min_angle_before_bargaining_corridor
            .clone())
    }

    fn update_min_angle_before_bargaining_corridor(
        &mut self,
        new_angle: AngleId,
    ) -> SimpleResult<()> {
        self.strategy_angles.min_angle_before_bargaining_corridor = Some(new_angle);
        Ok(())
    }

    fn get_max_angle_before_bargaining_corridor(&self) -> SimpleResult<Option<AngleId>> {
        Ok(self
            .strategy_angles
            .max_angle_before_bargaining_corridor
            .clone())
    }

    fn update_max_angle_before_bargaining_corridor(
        &mut self,
        new_angle: AngleId,
    ) -> SimpleResult<()> {
        self.strategy_angles.max_angle_before_bargaining_corridor = Some(new_angle);
        Ok(())
    }

    fn get_current_diff(&self) -> SimpleResult<Option<Diff>> {
        Ok(self.strategy_diffs.current_diff)
    }

    fn update_current_diff(&mut self, new_diff: Diff) -> SimpleResult<()> {
        self.strategy_diffs.current_diff = Some(new_diff);
        Ok(())
    }

    fn get_previous_diff(&self) -> SimpleResult<Option<Diff>> {
        Ok(self.strategy_diffs.previous_diff)
    }

    fn update_previous_diff(&mut self, new_diff: Diff) -> SimpleResult<()> {
        self.strategy_diffs.previous_diff = Some(new_diff);
        Ok(())
    }

    fn create_tick(&mut self, id: TickId, base_properties: TickBaseProperties) -> SimpleResult<()> {
        if let Some(tick) = self.tick_base_properties.get(&id) {
            return Err(SimpleError::new(format!(
                "a tick with an id {} already exists: {:?}",
                id, tick
            )));
        }

        self.tick_base_properties.insert(id, base_properties);

        Ok(())
    }

    fn get_tick_base_properties_by_id(
        &self,
        tick_id: &TickId,
    ) -> SimpleResult<Option<TickBaseProperties>> {
        Ok(self.tick_base_properties.get(tick_id).cloned())
    }

    fn get_all_ticks(&self) -> SimpleResult<HashSet<TickId>> {
        Ok(self.tick_base_properties.keys().cloned().collect())
    }

    fn create_candle(
        &mut self,
        id: CandleId,
        base_properties: CandleBaseProperties,
        edge_prices: CandleEdgePrices,
    ) -> SimpleResult<()> {
        if let Some(candle) = self.candle_base_properties.get(&id) {
            return Err(SimpleError::new(format!(
                "a candle with an id {} already exists: {:?}",
                id, candle
            )));
        }

        self.candle_base_properties
            .insert(id.clone(), base_properties);
        self.candle_edge_prices.insert(id, edge_prices);

        Ok(())
    }

    fn update_candle_base_properties(
        &mut self,
        id: CandleId,
        new_base_properties: CandleBaseProperties,
    ) -> SimpleResult<()> {
        if self.candle_base_properties.get(&id).is_none() {
            return Err(SimpleError::new(format!(
                "there is no candle with an id {} to update",
                id
            )));
        }

        self.candle_base_properties.insert(id, new_base_properties);

        Ok(())
    }

    fn get_candle_base_properties_by_id(
        &self,
        candle_id: &CandleId,
    ) -> SimpleResult<Option<CandleBaseProperties>> {
        Ok(self.candle_base_properties.get(candle_id).cloned())
    }

    fn get_candle_edge_prices_by_id(
        &self,
        candle_id: &CandleId,
    ) -> SimpleResult<Option<CandleEdgePrices>> {
        Ok(self.candle_edge_prices.get(candle_id).cloned())
    }

    fn get_all_candles(&self) -> SimpleResult<HashSet<CandleId>> {
        Ok(self.candle_base_properties.keys().cloned().collect())
    }

    fn get_current_tick(&self) -> SimpleResult<Option<TickId>> {
        Ok(self.strategy_ticks_candles.current_tick.clone())
    }

    fn update_current_tick(&mut self, tick_id: TickId) -> SimpleResult<()> {
        self.strategy_ticks_candles.current_tick = Some(tick_id);
        Ok(())
    }

    fn get_previous_tick(&self) -> SimpleResult<Option<TickId>> {
        Ok(self.strategy_ticks_candles.previous_tick.clone())
    }
    fn update_previous_tick(&mut self, tick_id: TickId) -> SimpleResult<()> {
        self.strategy_ticks_candles.previous_tick = Some(tick_id);
        Ok(())
    }

    fn get_current_candle(&self) -> SimpleResult<Option<CandleId>> {
        todo!()
    }
    fn update_current_candle(&mut self, candle_id: CandleId) -> SimpleResult<()> {
        self.strategy_ticks_candles.current_candle = Some(candle_id);
        Ok(())
    }

    fn get_previous_candle(&self) -> SimpleResult<Option<CandleId>> {
        todo!()
    }
    fn update_previous_candle(&mut self, candle_id: CandleId) -> SimpleResult<()> {
        self.strategy_ticks_candles.previous_candle = Some(candle_id);
        Ok(())
    }

    fn remove_unused_items(&mut self) -> SimpleResult<()> {
        // It's important to remove angles firstly. Otherwise it will block candles removal
        self.remove_unused_angles();
        self.remove_unused_candles();
        self.remove_unused_ticks();

        Ok(())
    }

    fn add_ratio_setting(
        &mut self,
        name: SettingProgramName,
        value: RatioSettingValue,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn add_point_setting(
        &mut self,
        name: SettingProgramName,
        value: PointSettingValue,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_ratio_setting(
        &self,
        name: &SettingProgramName,
    ) -> SimpleResult<Option<RatioSettingValue>> {
        todo!()
    }

    fn get_point_setting(
        &self,
        name: &SettingProgramName,
    ) -> SimpleResult<Option<PointSettingValue>> {
        todo!()
    }

    fn create_working_level(
        &mut self,
        id: WLId,
        base_properties: crate::step::utils::entities::working_levels::WorkingLevelBaseProperties,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_working_level_base_properties_by_id(
        &self,
        id: &WLId,
    ) -> SimpleResult<
        Option<crate::step::utils::entities::working_levels::WorkingLevelBaseProperties>,
    > {
        todo!()
    }

    fn update_working_level_base_properties(
        &mut self,
        id: WLId,
        new_base_properties: crate::step::utils::entities::working_levels::WorkingLevelBaseProperties,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn move_working_level_to_active(&mut self, id: &WLId) -> SimpleResult<()> {
        todo!()
    }

    fn move_working_level_to_removed(&mut self, id: &WLId) -> SimpleResult<()> {
        todo!()
    }

    fn remove_working_level(&mut self, id: &WLId) -> SimpleResult<()> {
        todo!()
    }

    fn get_created_working_levels(&self) -> SimpleResult<HashSet<WLId>> {
        todo!()
    }

    fn get_active_working_levels(&self) -> SimpleResult<HashSet<WLId>> {
        todo!()
    }

    fn get_removed_working_levels(&self) -> SimpleResult<HashSet<WLId>> {
        todo!()
    }

    fn add_candle_to_working_level_corridor(
        &mut self,
        working_level_id: WLId,
        candle_id: CandleId,
        corridor_type: CorridorType,
    ) -> SimpleResult<()> {
        let already_exists_error = format!("a candle with an id {} already exists in a small corridor of a working level with id {}", candle_id, working_level_id);

        let new_working_level_corridor_candle = WorkingLevelCorridorCandle {
            working_level_id,
            candle_id,
        };

        let working_level_corridors = match corridor_type {
            CorridorType::Small => &mut self.working_level_small_corridors,
            CorridorType::Big => &mut self.working_level_big_corridors,
        };

        if working_level_corridors.contains(&new_working_level_corridor_candle) {
            return Err(SimpleError::new(already_exists_error));
        }

        working_level_corridors.push(new_working_level_corridor_candle);

        Ok(())
    }

    fn get_candles_of_working_level_corridor(
        &self,
        working_level_id: &WLId,
        corridor_type: CorridorType,
    ) -> SimpleResult<HashSet<CandleId>> {
        todo!()
    }

    fn add_max_crossing_value_to_working_level(
        &mut self,
        working_level_id: WLId,
        value: crate::step::utils::entities::working_levels::WLMaxCrossingValue,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn update_max_crossing_value_of_working_level(
        &mut self,
        working_level_id: WLId,
        new_value: crate::step::utils::entities::working_levels::WLMaxCrossingValue,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn add_working_level_to_list_of_levels_with_moved_take_profits(
        &mut self,
        working_level_id: WLId,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_list_of_working_levels_with_moved_take_profits(&self) -> SimpleResult<HashSet<WLId>> {
        todo!()
    }

    fn create_order(
        &mut self,
        id: OrderId,
        base_prices: base::entities::OrderBasePrices,
        base_properties: OrderBaseProperties,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_order_base_prices_by_id(
        &self,
        id: &OrderId,
    ) -> SimpleResult<Option<base::entities::OrderBasePrices>> {
        todo!()
    }

    fn get_order_base_properties_by_id(
        &self,
        id: &OrderId,
    ) -> SimpleResult<Option<OrderBaseProperties>> {
        todo!()
    }

    fn remove_order(&mut self, id: &OrderId) -> SimpleResult<()> {
        todo!()
    }

    fn add_order_to_working_level_chain_of_orders(
        &mut self,
        working_level_id: WLId,
        order_id: OrderId,
    ) -> SimpleResult<()> {
        todo!()
    }

    fn get_orders_of_working_level(
        &self,
        working_level_id: &WLId,
    ) -> SimpleResult<HashSet<OrderId>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_unused_items() {
        let mut store: InMemoryStepBacktestingStore = Default::default();

        for i in 1..=4 {
            let _ = store.create_tick(i.to_string(), Default::default());
        }

        let _ = store.update_current_tick(String::from("1"));
        let _ = store.update_previous_tick(String::from("2"));

        for i in 1..=10 {
            let _ = store.create_candle(i.to_string(), Default::default(), Default::default());
        }

        let _ = store.update_current_candle(String::from("1"));
        let _ = store.update_previous_candle(String::from("2"));

        let _ = store.add_candle_to_working_level_corridor(
            String::from("1"),
            String::from("3"),
            CorridorType::Small,
        );

        let _ = store.add_candle_to_working_level_corridor(
            String::from("1"),
            String::from("4"),
            CorridorType::Big,
        );

        for i in 1..=10 {
            let _ = store.create_angle(
                i.to_string(),
                Angle {
                    candle_id: i.to_string(),
                    r#type: Level::Min,
                },
            );
        }

        let _ =
            store.update_angle_of_second_level_after_bargaining_tendency_change(String::from("1"));
        let _ = store.update_tendency_change_angle(String::from("2"));
        let _ = store.update_min_angle(String::from("3"));
        let _ = store.update_max_angle(String::from("4"));
        let _ = store.update_virtual_min_angle(String::from("5"));
        let _ = store.update_virtual_max_angle(String::from("6"));
        let _ = store.update_min_angle_before_bargaining_corridor(String::from("7"));
        let _ = store.update_max_angle_before_bargaining_corridor(String::from("8"));

        let _ = store.remove_unused_items();

        let mut left_ticks = HashSet::new();
        left_ticks.insert(String::from("1"));
        left_ticks.insert(String::from("2"));

        assert!(store
            .get_all_ticks()
            .unwrap()
            .symmetric_difference(&left_ticks)
            .collect::<HashSet<&TickId>>()
            .is_empty());

        let mut left_candles = HashSet::new();
        left_candles.extend((1..=8).map(|i| i.to_string()));

        assert!(store
            .get_all_candles()
            .unwrap()
            .symmetric_difference(&left_candles)
            .collect::<HashSet<&CandleId>>()
            .is_empty());

        let mut left_angles = HashSet::new();
        left_angles.extend((1..=8).map(|i| i.to_string()));

        assert!(store
            .get_all_angles()
            .unwrap()
            .symmetric_difference(&left_angles)
            .collect::<HashSet<&AngleId>>()
            .is_empty())
    }
}
