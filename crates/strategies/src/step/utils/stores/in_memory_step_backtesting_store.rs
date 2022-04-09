use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};

use backtesting::{
    backtesting_base_store::BacktestingBaseStore, BacktestingConfig, BacktestingLowLevelData,
    Balance, Leverage, Spread, Trades, Units,
};
use base::entities::{
    candle::CandleId, order::OrderId, tick::TickId, CandleBaseProperties, CandleEdgePrices,
    MovementType, OrderBasePrices, OrderBaseProperties, TickBaseProperties,
};

use crate::step::utils::entities::working_levels::{CorridorType, WLMaxCrossingValue};
use crate::step::utils::entities::{
    angles::{Angle, AngleId},
    strategies::{
        BacktestingStatisticNumber, BacktestingStatistics, StrategyAngles, StrategyBaseConfig,
        StrategyDiffs, StrategyTicksCandles, Symbol,
    },
    working_levels::{WLId, WorkingLevelBaseProperties},
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

    working_level_max_crossing_values: HashMap<WLId, WLMaxCrossingValue>,
    working_levels_with_moved_take_profits: HashSet<WLId>,

    created_working_levels: HashSet<WLId>,
    active_working_levels: HashSet<WLId>,
    removed_working_levels: HashSet<WLId>,

    working_level_small_corridors: HashMap<WLId, HashSet<CandleId>>,
    working_level_big_corridors: HashMap<WLId, HashSet<CandleId>>,
    corridor_candles: HashSet<CandleId>,

    working_level_chain_of_orders: HashMap<WLId, HashSet<OrderId>>,

    order_base_prices: HashMap<OrderId, OrderBasePrices>,
    order_base_properties: HashMap<OrderId, OrderBaseProperties>,

    strategy_base_config: StrategyBaseConfig,
    strategy_angles: StrategyAngles,
    strategy_diffs: StrategyDiffs,
    strategy_ticks_candles: StrategyTicksCandles,

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
        let candle_in_corridors = |candle_id: &CandleId| self.corridor_candles.contains(candle_id);
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
    fn get_number_of_working_levels(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self.backtesting_statistics.number_of_working_levels)
    }

    fn update_number_of_working_levels(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics.number_of_working_levels = new_number;

        Ok(())
    }

    fn get_number_of_tendency_changes(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self.backtesting_statistics.number_of_tendency_changes)
    }

    fn update_number_of_tendency_changes(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics.number_of_tendency_changes = new_number;

        Ok(())
    }

    fn get_deleted_by_being_close_to_another_one(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_being_close_to_another_one)
    }

    fn update_deleted_by_being_close_to_another_one(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_being_close_to_another_one = new_number;

        Ok(())
    }

    fn get_deleted_by_another_active_chain_of_orders(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_another_active_chain_of_orders)
    }

    fn update_deleted_by_another_active_chain_of_orders(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_another_active_chain_of_orders = new_number;

        Ok(())
    }

    fn get_deleted_by_expiration_by_distance(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_expiration_by_distance)
    }

    fn update_deleted_by_expiration_by_distance(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_expiration_by_distance = new_number;

        Ok(())
    }

    fn get_deleted_by_expiration_by_time(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self.backtesting_statistics.deleted_by_expiration_by_time)
    }

    fn update_deleted_by_expiration_by_time(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics.deleted_by_expiration_by_time = new_number;

        Ok(())
    }

    fn get_deleted_by_price_being_beyond_stop_loss(&self) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_price_being_beyond_stop_loss)
    }

    fn update_deleted_by_price_being_beyond_stop_loss(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_price_being_beyond_stop_loss = new_number;

        Ok(())
    }

    fn get_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(
        &self,
    ) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing)
    }

    fn update_deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing =
            new_number;

        Ok(())
    }

    fn get_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(
        &self,
    ) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing)
    }

    fn update_deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing =
            new_number;

        Ok(())
    }

    fn get_deleted_by_exceeding_activation_crossing_distance(
        &self,
    ) -> Result<BacktestingStatisticNumber> {
        Ok(self
            .backtesting_statistics
            .deleted_by_exceeding_activation_crossing_distance)
    }

    fn update_deleted_by_exceeding_activation_crossing_distance(
        &mut self,
        new_number: BacktestingStatisticNumber,
    ) -> Result<()> {
        self.backtesting_statistics
            .deleted_by_exceeding_activation_crossing_distance = new_number;

        Ok(())
    }
}

impl BacktestingBaseStore for InMemoryStepBacktestingStore {
    fn get_initial_balance(&self) -> Result<Balance> {
        Ok(self.backtesting_low_level_data.initial_balance)
    }

    fn get_processing_balance(&self) -> Result<Balance> {
        Ok(self.backtesting_low_level_data.processing_balance)
    }

    fn update_processing_balance(&mut self, new_processing_balance: Balance) -> Result<()> {
        self.backtesting_low_level_data.processing_balance = new_processing_balance;

        Ok(())
    }

    fn get_real_balance(&self) -> Result<Balance> {
        Ok(self.backtesting_low_level_data.real_balance)
    }

    fn update_real_balance(&mut self, new_real_balance: Balance) -> Result<()> {
        self.backtesting_low_level_data.real_balance = new_real_balance;

        Ok(())
    }

    fn get_units(&self) -> Result<Units> {
        Ok(self.backtesting_low_level_data.units)
    }

    fn update_units(&mut self, new_number_of_units: Units) -> Result<()> {
        self.backtesting_low_level_data.units = new_number_of_units;

        Ok(())
    }

    fn get_trades(&self) -> Result<Trades> {
        Ok(self.backtesting_low_level_data.trades)
    }

    fn update_trades(&mut self, new_number_of_trades: Trades) -> Result<()> {
        self.backtesting_low_level_data.trades = new_number_of_trades;

        Ok(())
    }

    fn get_leverage(&self) -> Result<Leverage> {
        Ok(self.backtesting_config.leverage)
    }

    fn get_use_spread(&self) -> Result<bool> {
        Ok(self.backtesting_config.use_spread)
    }

    fn get_spread(&self) -> Result<Spread> {
        Ok(self.backtesting_config.spread)
    }
}

impl StepBaseStore for InMemoryStepBacktestingStore {
    fn get_symbol(&self) -> Result<Symbol> {
        Ok(self.strategy_base_config.symbol)
    }

    fn get_tendency(&self) -> Result<MovementType> {
        Ok(self.strategy_base_config.tendency)
    }

    fn update_tendency(&mut self, value: MovementType) -> Result<()> {
        self.strategy_base_config.tendency = value;
        Ok(())
    }

    fn get_tendency_changed_on_crossing_bargaining_corridor(&self) -> Result<bool> {
        Ok(self
            .strategy_base_config
            .tendency_changed_on_crossing_bargaining_corridor)
    }

    fn update_tendency_changed_on_crossing_bargaining_corridor(
        &mut self,
        value: bool,
    ) -> Result<()> {
        self.strategy_base_config
            .tendency_changed_on_crossing_bargaining_corridor = value;
        Ok(())
    }

    fn get_second_level_after_bargaining_tendency_change_is_created(&self) -> Result<bool> {
        Ok(self
            .strategy_base_config
            .second_level_after_bargaining_tendency_change_is_created)
    }

    fn update_second_level_after_bargaining_tendency_change_is_created(
        &mut self,
        value: bool,
    ) -> Result<()> {
        self.strategy_base_config
            .second_level_after_bargaining_tendency_change_is_created = value;
        Ok(())
    }

    fn get_skip_creating_new_working_level(&self) -> Result<bool> {
        Ok(self.strategy_base_config.skip_creating_new_working_level)
    }

    fn update_skip_creating_new_working_level(&mut self, value: bool) -> Result<()> {
        self.strategy_base_config.skip_creating_new_working_level = value;
        Ok(())
    }

    fn get_no_trading_mode(&self) -> Result<bool> {
        Ok(self.strategy_base_config.no_trading_mode)
    }

    fn update_no_trading_mode(&mut self, value: bool) -> Result<()> {
        self.strategy_base_config.no_trading_mode = value;
        Ok(())
    }

    fn create_angle(&mut self, id: AngleId, new_angle: Angle) -> Result<()> {
        if let Some(angle) = self.angles.get(&id) {
            bail!("an angle with an id {} already exists: {:?}", id, angle);
        }

        self.angles.insert(id, new_angle);

        Ok(())
    }

    fn get_angle_by_id(&self, id: &str) -> Result<Option<Angle>> {
        Ok(self.angles.get(id).cloned())
    }

    fn update_angle(&mut self, id: &str, new_angle: Angle) -> Result<()> {
        if self.angles.get(id).is_none() {
            bail!("there is no angle with an id {}", id);
        }

        self.angles.insert(id.to_string(), new_angle);

        Ok(())
    }

    fn get_all_angles(&self) -> Result<HashSet<AngleId>> {
        Ok(self.angles.keys().cloned().collect())
    }

    fn get_angle_of_second_level_after_bargaining_tendency_change(
        &self,
    ) -> Result<Option<AngleId>> {
        Ok(self
            .strategy_angles
            .angle_of_second_level_after_bargaining_tendency_change
            .clone())
    }

    fn update_angle_of_second_level_after_bargaining_tendency_change(
        &mut self,
        new_angle: AngleId,
    ) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles
            .angle_of_second_level_after_bargaining_tendency_change = Some(new_angle);
        Ok(())
    }

    fn get_tendency_change_angle(&self) -> Result<Option<AngleId>> {
        Ok(self.strategy_angles.tendency_change_angle.clone())
    }

    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.tendency_change_angle = Some(new_angle);
        Ok(())
    }

    fn get_min_angle(&self) -> Result<Option<AngleId>> {
        Ok(self.strategy_angles.min_angle.clone())
    }

    fn update_min_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.min_angle = Some(new_angle);
        Ok(())
    }

    fn get_virtual_min_angle(&self) -> Result<Option<AngleId>> {
        Ok(self.strategy_angles.virtual_min_angle.clone())
    }

    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.virtual_min_angle = Some(new_angle);
        Ok(())
    }

    fn get_max_angle(&self) -> Result<Option<AngleId>> {
        Ok(self.strategy_angles.max_angle.clone())
    }

    fn update_max_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.max_angle = Some(new_angle);
        Ok(())
    }

    fn get_virtual_max_angle(&self) -> Result<Option<AngleId>> {
        Ok(self.strategy_angles.virutal_max_angle.clone())
    }

    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.virutal_max_angle = Some(new_angle);
        Ok(())
    }

    fn get_min_angle_before_bargaining_corridor(&self) -> Result<Option<AngleId>> {
        Ok(self
            .strategy_angles
            .min_angle_before_bargaining_corridor
            .clone())
    }

    fn update_min_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.min_angle_before_bargaining_corridor = Some(new_angle);
        Ok(())
    }

    fn get_max_angle_before_bargaining_corridor(&self) -> Result<Option<AngleId>> {
        Ok(self
            .strategy_angles
            .max_angle_before_bargaining_corridor
            .clone())
    }

    fn update_max_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.max_angle_before_bargaining_corridor = Some(new_angle);
        Ok(())
    }

    fn get_current_diff(&self) -> Result<Option<Diff>> {
        Ok(self.strategy_diffs.current_diff)
    }

    fn update_current_diff(&mut self, new_diff: Diff) -> Result<()> {
        self.strategy_diffs.current_diff = Some(new_diff);
        Ok(())
    }

    fn get_previous_diff(&self) -> Result<Option<Diff>> {
        Ok(self.strategy_diffs.previous_diff)
    }

    fn update_previous_diff(&mut self, new_diff: Diff) -> Result<()> {
        self.strategy_diffs.previous_diff = Some(new_diff);
        Ok(())
    }

    fn create_tick(&mut self, id: TickId, base_properties: TickBaseProperties) -> Result<()> {
        if let Some(tick) = self.tick_base_properties.get(&id) {
            bail!("a tick with an id {} already exists: {:?}", id, tick);
        }

        self.tick_base_properties.insert(id, base_properties);

        Ok(())
    }

    fn get_tick_base_properties_by_id(&self, tick_id: &str) -> Result<Option<TickBaseProperties>> {
        Ok(self.tick_base_properties.get(tick_id).cloned())
    }

    fn get_all_ticks(&self) -> Result<HashSet<TickId>> {
        Ok(self.tick_base_properties.keys().cloned().collect())
    }

    fn create_candle(
        &mut self,
        id: CandleId,
        base_properties: CandleBaseProperties,
        edge_prices: CandleEdgePrices,
    ) -> Result<()> {
        if let Some(candle) = self.candle_base_properties.get(&id) {
            bail!("a candle with an id {} already exists: {:?}", id, candle);
        }

        self.candle_base_properties
            .insert(id.clone(), base_properties);
        self.candle_edge_prices.insert(id, edge_prices);

        Ok(())
    }

    fn update_candle_base_properties(
        &mut self,
        id: &str,
        new_base_properties: CandleBaseProperties,
    ) -> Result<()> {
        if self.candle_base_properties.get(id).is_none() {
            bail!("there is no candle with an id {} to update", id);
        }

        self.candle_base_properties
            .insert(id.to_string(), new_base_properties);

        Ok(())
    }

    fn get_candle_base_properties_by_id(
        &self,
        candle_id: &str,
    ) -> Result<Option<CandleBaseProperties>> {
        Ok(self.candle_base_properties.get(candle_id).cloned())
    }

    fn get_candle_edge_prices_by_id(
        &self,
        candle_id: &CandleId,
    ) -> Result<Option<CandleEdgePrices>> {
        Ok(self.candle_edge_prices.get(candle_id).cloned())
    }

    fn get_all_candles(&self) -> Result<HashSet<CandleId>> {
        Ok(self.candle_base_properties.keys().cloned().collect())
    }

    fn get_current_tick(&self) -> Result<Option<TickId>> {
        Ok(self.strategy_ticks_candles.current_tick.clone())
    }

    fn update_current_tick(&mut self, tick_id: TickId) -> Result<()> {
        if !self.tick_base_properties.contains_key(&tick_id) {
            bail!("a tick with an id {} doesn't exist", tick_id);
        }

        self.strategy_ticks_candles.current_tick = Some(tick_id);
        Ok(())
    }

    fn get_previous_tick(&self) -> Result<Option<TickId>> {
        Ok(self.strategy_ticks_candles.previous_tick.clone())
    }
    fn update_previous_tick(&mut self, tick_id: TickId) -> Result<()> {
        if !self.tick_base_properties.contains_key(&tick_id) {
            bail!("a tick with an id {} doesn't exist", tick_id);
        }

        self.strategy_ticks_candles.previous_tick = Some(tick_id);
        Ok(())
    }

    fn get_current_candle(&self) -> Result<Option<CandleId>> {
        Ok(self.strategy_ticks_candles.current_candle.clone())
    }
    fn update_current_candle(&mut self, candle_id: CandleId) -> Result<()> {
        if !self.candle_base_properties.contains_key(&candle_id) {
            bail!("a candle with an id {} doesn't exist", candle_id);
        }

        self.strategy_ticks_candles.current_candle = Some(candle_id);
        Ok(())
    }

    fn get_previous_candle(&self) -> Result<Option<CandleId>> {
        Ok(self.strategy_ticks_candles.previous_candle.clone())
    }
    fn update_previous_candle(&mut self, candle_id: CandleId) -> Result<()> {
        if !self.candle_base_properties.contains_key(&candle_id) {
            bail!("a candle with an id {} doesn't exist", candle_id);
        }

        self.strategy_ticks_candles.previous_candle = Some(candle_id);
        Ok(())
    }

    fn remove_unused_items(&mut self) -> Result<()> {
        // It's important to remove angles firstly. Otherwise it will block candles removal
        self.remove_unused_angles();
        self.remove_unused_candles();
        self.remove_unused_ticks();

        Ok(())
    }

    fn create_working_level(
        &mut self,
        id: WLId,
        base_properties: WorkingLevelBaseProperties,
    ) -> Result<()> {
        if self.working_level_base_properties.contains_key(&id) {
            bail!("a working level with an id {} already exists", id);
        }

        self.working_level_base_properties
            .insert(id.clone(), base_properties);

        self.created_working_levels.insert(id);

        Ok(())
    }

    fn get_working_level_base_properties_by_id(
        &self,
        id: &str,
    ) -> Result<Option<WorkingLevelBaseProperties>> {
        Ok(self.working_level_base_properties.get(id).cloned())
    }

    fn update_working_level_base_properties(
        &mut self,
        id: &str,
        new_base_properties: WorkingLevelBaseProperties,
    ) -> Result<()> {
        if !self.working_level_base_properties.contains_key(id) {
            bail!("a working level with an id {} is not found", id);
        }

        self.working_level_base_properties
            .insert(id.to_string(), new_base_properties);

        Ok(())
    }

    fn move_working_level_to_active(&mut self, id: &str) -> Result<()> {
        if !self.created_working_levels.contains(id) {
            bail!("can't move a working level with an id {} to active levels, because the level is not found in created levels", id);
        }

        self.created_working_levels.remove(id);
        self.active_working_levels.insert(id.to_string());

        Ok(())
    }

    fn move_working_level_to_removed(&mut self, id: &str) -> Result<()> {
        let existed_in_created = self.created_working_levels.remove(id);
        let existed_in_active = self.active_working_levels.remove(id);

        if !existed_in_created && !existed_in_active {
            bail!("can't move a working level with an id {} to removed levels, because it wasn't found neither in created not in active levels", id);
        }

        self.removed_working_levels.insert(id.to_string());

        Ok(())
    }

    fn remove_working_level(&mut self, id: &str) -> Result<()> {
        if self.working_level_base_properties.remove(id).is_none() {
            bail!("a working level with an id {} doesn't exist", id);
        }

        self.working_level_big_corridors.remove(id);
        self.working_level_small_corridors.remove(id);

        self.working_level_max_crossing_values.remove(id);

        if let Some(orders) = self.working_level_chain_of_orders.remove(id) {
            for order in orders.iter() {
                let _ = self.remove_order(order)?;
            }
        }

        self.working_levels_with_moved_take_profits.remove(id);

        self.created_working_levels.remove(id);
        self.active_working_levels.remove(id);
        self.removed_working_levels.remove(id);

        Ok(())
    }

    fn get_created_working_levels(&self) -> Result<HashSet<WLId>> {
        Ok(self.created_working_levels.clone())
    }

    fn get_active_working_levels(&self) -> Result<HashSet<WLId>> {
        Ok(self.active_working_levels.clone())
    }

    fn get_removed_working_levels(&self) -> Result<HashSet<WLId>> {
        Ok(self.removed_working_levels.clone())
    }

    fn add_candle_to_working_level_corridor(
        &mut self,
        working_level_id: &str,
        candle_id: CandleId,
        corridor_type: CorridorType,
    ) -> Result<()> {
        if !self
            .working_level_base_properties
            .contains_key(working_level_id)
        {
            bail!(
                "a working level with an id {} doesn't exist",
                working_level_id
            );
        }

        if !self.candle_base_properties.contains_key(&candle_id) {
            bail!("a candle with an id {} doesn't exist", candle_id);
        }

        let working_level_corridors = match corridor_type {
            CorridorType::Small => &mut self.working_level_small_corridors,
            CorridorType::Big => &mut self.working_level_big_corridors,
        };

        let candles = working_level_corridors
            .entry(working_level_id.to_string())
            .or_default();

        if candles.contains(&candle_id) {
            bail!("a candle with an id {} already exists in a {:?} corridor of a working level with id {}", candle_id, corridor_type, working_level_id);
        }

        self.corridor_candles.insert(candle_id.clone());

        candles.insert(candle_id);

        Ok(())
    }

    fn get_candles_of_working_level_corridor(
        &self,
        working_level_id: &str,
        corridor_type: CorridorType,
    ) -> Result<Option<HashSet<CandleId>>> {
        match corridor_type {
            CorridorType::Small => Ok(self
                .working_level_small_corridors
                .get(working_level_id)
                .cloned()),
            CorridorType::Big => Ok(self
                .working_level_big_corridors
                .get(working_level_id)
                .cloned()),
        }
    }

    fn update_max_crossing_value_of_working_level(
        &mut self,
        working_level_id: &str,
        new_value: WLMaxCrossingValue,
    ) -> Result<()> {
        if !self
            .working_level_base_properties
            .contains_key(working_level_id)
        {
            bail!(
                "a working level with an id {} doesn't exist",
                working_level_id
            );
        }

        self.working_level_max_crossing_values
            .insert(working_level_id.to_string(), new_value);
        Ok(())
    }

    fn get_max_crossing_value_of_working_level(
        &self,
        working_level_id: &str,
    ) -> Result<Option<WLMaxCrossingValue>> {
        Ok(self
            .working_level_max_crossing_values
            .get(working_level_id)
            .cloned())
    }

    fn move_take_profits_of_level(&mut self, working_level_id: &str) -> Result<()> {
        if !self
            .working_level_base_properties
            .contains_key(working_level_id)
        {
            bail!(
                "a working level with an id {} doesn't exist",
                working_level_id
            );
        }

        let was_not_present = self
            .working_levels_with_moved_take_profits
            .insert(working_level_id.to_string());

        if !was_not_present {
            bail!(
                "take profits are already moved for a working level with an id {}",
                working_level_id
            );
        }

        Ok(())
    }

    fn are_take_profits_of_level_moved(&self, working_level_id: &str) -> Result<bool> {
        Ok(self
            .working_levels_with_moved_take_profits
            .contains(working_level_id))
    }

    fn create_order(
        &mut self,
        id: OrderId,
        base_prices: OrderBasePrices,
        base_properties: OrderBaseProperties,
    ) -> Result<()> {
        if self.order_base_prices.contains_key(&id) {
            bail!("an order with an id {} already exists", id);
        }

        self.order_base_properties
            .insert(id.clone(), base_properties);
        self.order_base_prices.insert(id, base_prices);

        Ok(())
    }

    fn get_order_base_prices_by_id(&self, id: &str) -> Result<Option<OrderBasePrices>> {
        Ok(self.order_base_prices.get(id).cloned())
    }

    fn get_order_base_properties_by_id(&self, id: &str) -> Result<Option<OrderBaseProperties>> {
        Ok(self.order_base_properties.get(id).cloned())
    }

    fn remove_order(&mut self, id: &str) -> Result<()> {
        if self.order_base_properties.remove(id).is_none() {
            bail!("can't remove a non-existent order with an id {}", id);
        }

        self.order_base_prices.remove(id);

        Ok(())
    }

    fn add_order_to_working_level_chain_of_orders(
        &mut self,
        working_level_id: &str,
        order_id: OrderId,
    ) -> Result<()> {
        if !self
            .working_level_base_properties
            .contains_key(working_level_id)
        {
            bail!(
                "a working level with an id {} doesn't exist",
                working_level_id
            );
        }

        if !self.order_base_properties.contains_key(&order_id) {
            bail!("an order with an id {} doesn't exist", order_id);
        }

        let set_of_orders = self
            .working_level_chain_of_orders
            .entry(working_level_id.to_string())
            .or_default();

        let did_not_exist = set_of_orders.insert(order_id.clone());
        if !did_not_exist {
            bail!("an order with an id {} already exists in a chain of orders of a working level with an id {}", order_id, working_level_id);
        }

        Ok(())
    }

    fn get_working_level_chain_of_orders(
        &self,
        working_level_id: &str,
    ) -> Result<Option<HashSet<OrderId>>> {
        Ok(self
            .working_level_chain_of_orders
            .get(working_level_id)
            .cloned())
    }
}
