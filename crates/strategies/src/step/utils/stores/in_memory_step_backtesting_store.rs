use std::collections::{HashMap, HashSet};

use anyhow::{bail, Context, Result};

use base::entities::candle::BasicCandleProperties;
use base::entities::Item;
use base::entities::{candle::CandleId, tick::TickId, BasicTickProperties};

use crate::step::utils::entities::order::{BasicOrderProperties, OrderId, OrderStatus};
use crate::step::utils::entities::working_levels::{CorridorType, WLMaxCrossingValue};
use crate::step::utils::entities::{
    angle::{AngleId, BasicAngleProperties},
    working_levels::{BasicWLProperties, WLId},
};
use crate::step::utils::stores::{StepStrategyAngles, StepStrategyTicksCandles};

use super::angle_store::AngleStore;
use super::candle_store::CandleStore;
use super::tick_store::TickStore;
use super::working_level_store::WorkingLevelStore;

#[derive(Default)]
pub struct InMemoryStepBacktestingStore {
    candles: HashMap<CandleId, Item<CandleId, BasicCandleProperties>>,
    ticks: HashMap<TickId, Item<TickId, BasicTickProperties>>,
    angles: HashMap<AngleId, Item<AngleId, BasicAngleProperties>>,

    working_levels: HashMap<WLId, Item<WLId, BasicWLProperties>>,

    working_level_max_crossing_values: HashMap<WLId, WLMaxCrossingValue>,
    working_levels_with_moved_take_profits: HashSet<WLId>,

    created_working_levels: HashSet<WLId>,
    active_working_levels: HashSet<WLId>,
    removed_working_levels: HashSet<WLId>,

    working_level_small_corridors: HashMap<WLId, HashSet<CandleId>>,
    working_level_big_corridors: HashMap<WLId, HashSet<CandleId>>,
    corridor_candles: HashSet<CandleId>,

    working_level_chain_of_orders: HashMap<WLId, HashSet<OrderId>>,
    orders: HashMap<OrderId, Item<OrderId, BasicOrderProperties>>,

    strategy_angles: StepStrategyAngles,
    strategy_ticks_candles: StepStrategyTicksCandles,
}

impl TickStore for InMemoryStepBacktestingStore {
    type TickProperties = BasicTickProperties;

    fn create_tick(&mut self, properties: Self::TickProperties) -> Result<TickId> {
        let id = xid::new().to_string();

        let new_tick = Item {
            id: id.clone(),
            props: properties,
        };

        self.ticks.insert(id.clone(), new_tick);

        Ok(id)
    }

    fn get_tick_by_id(&self, tick_id: &str) -> Result<Option<Item<TickId, Self::TickProperties>>> {
        Ok(self.ticks.get(tick_id).cloned())
    }

    fn get_current_tick(&self) -> Result<Option<Item<TickId, Self::TickProperties>>> {
        let tick_id = self.strategy_ticks_candles.current_tick.as_ref();

        let tick_id = match tick_id {
            None => return Ok(None),
            Some(tick_id) => tick_id,
        };

        self.get_tick_by_id(tick_id)
    }

    fn update_current_tick(&mut self, tick_id: TickId) -> Result<()> {
        if !self.ticks.contains_key(&tick_id) {
            bail!("a tick with an id {} doesn't exist", tick_id);
        }

        self.strategy_ticks_candles.current_tick = Some(tick_id);
        Ok(())
    }

    fn get_previous_tick(&self) -> Result<Option<Item<TickId, Self::TickProperties>>> {
        let tick_id = self.strategy_ticks_candles.previous_tick.as_ref();

        let tick_id = match tick_id {
            None => return Ok(None),
            Some(tick_id) => tick_id,
        };

        self.get_tick_by_id(tick_id)
    }

    fn update_previous_tick(&mut self, tick_id: TickId) -> Result<()> {
        if !self.ticks.contains_key(&tick_id) {
            bail!("a tick with an id {} doesn't exist", tick_id);
        }

        self.strategy_ticks_candles.previous_tick = Some(tick_id);

        Ok(())
    }
}

impl CandleStore for InMemoryStepBacktestingStore {
    type CandleProperties = BasicCandleProperties;

    fn create_candle(&mut self, properties: Self::CandleProperties) -> Result<CandleId> {
        let id = xid::new().to_string();

        let new_candle = Item {
            id: id.clone(),
            props: properties,
        };

        self.candles.insert(id.clone(), new_candle);

        Ok(id)
    }

    fn get_candle_by_id(
        &self,
        candle_id: &str,
    ) -> Result<Option<Item<CandleId, Self::CandleProperties>>> {
        Ok(self.candles.get(candle_id).cloned())
    }

    fn get_current_candle(&self) -> Result<Option<Item<CandleId, Self::CandleProperties>>> {
        let candle_id = self.strategy_ticks_candles.current_candle.as_ref();

        let candle_id = match candle_id {
            None => return Ok(None),
            Some(candle_id) => candle_id,
        };

        self.get_candle_by_id(candle_id)
    }

    fn update_current_candle(&mut self, candle_id: CandleId) -> Result<()> {
        if !self.candles.contains_key(&candle_id) {
            bail!("a candle with an id {} doesn't exist", candle_id);
        }

        self.strategy_ticks_candles.current_candle = Some(candle_id);
        Ok(())
    }

    fn get_previous_candle(&self) -> Result<Option<Item<CandleId, Self::CandleProperties>>> {
        let candle_id = self.strategy_ticks_candles.previous_candle.as_ref();

        let candle_id = match candle_id {
            None => return Ok(None),
            Some(candle_id) => candle_id,
        };

        self.get_candle_by_id(candle_id)
    }

    fn update_previous_candle(&mut self, candle_id: CandleId) -> Result<()> {
        if !self.candles.contains_key(&candle_id) {
            bail!("a candle with an id {} doesn't exist", candle_id);
        }

        self.strategy_ticks_candles.previous_candle = Some(candle_id);
        Ok(())
    }
}

impl AngleStore for InMemoryStepBacktestingStore {
    type AngleProperties = BasicAngleProperties;

    fn create_angle(&mut self, properties: Self::AngleProperties) -> Result<AngleId> {
        let id = xid::new().to_string();

        let new_angle = Item {
            id: id.clone(),
            props: properties,
        };

        self.angles.insert(id.clone(), new_angle);

        Ok(id)
    }

    fn get_angle_by_id(&self, id: &str) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        Ok(self.angles.get(id).cloned())
    }

    fn get_angle_of_second_level_after_bargaining_tendency_change(
        &self,
    ) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self
            .strategy_angles
            .angle_of_second_level_after_bargaining_tendency_change
            .as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
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

    fn get_tendency_change_angle(&self) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self.strategy_angles.tendency_change_angle.as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_tendency_change_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.tendency_change_angle = Some(new_angle);
        Ok(())
    }

    fn get_min_angle(&self) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self.strategy_angles.min_angle.as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_min_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.min_angle = Some(new_angle);
        Ok(())
    }

    fn get_virtual_min_angle(&self) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self.strategy_angles.virtual_min_angle.as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_virtual_min_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.virtual_min_angle = Some(new_angle);
        Ok(())
    }

    fn get_max_angle(&self) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self.strategy_angles.max_angle.as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_max_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.max_angle = Some(new_angle);
        Ok(())
    }

    fn get_virtual_max_angle(&self) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self.strategy_angles.virtual_max_angle.as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_virtual_max_angle(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.virtual_max_angle = Some(new_angle);
        Ok(())
    }

    fn get_min_angle_before_bargaining_corridor(
        &self,
    ) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self
            .strategy_angles
            .min_angle_before_bargaining_corridor
            .as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_min_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.min_angle_before_bargaining_corridor = Some(new_angle);
        Ok(())
    }

    fn get_max_angle_before_bargaining_corridor(
        &self,
    ) -> Result<Option<Item<AngleId, Self::AngleProperties>>> {
        let angle_id = self
            .strategy_angles
            .max_angle_before_bargaining_corridor
            .as_ref();

        let angle_id = match angle_id {
            None => return Ok(None),
            Some(angle_id) => angle_id,
        };

        self.get_angle_by_id(angle_id)
    }

    fn update_max_angle_before_bargaining_corridor(&mut self, new_angle: AngleId) -> Result<()> {
        if !self.angles.contains_key(&new_angle) {
            bail!("an angle with an id {} doesn't exist", new_angle);
        }

        self.strategy_angles.max_angle_before_bargaining_corridor = Some(new_angle);
        Ok(())
    }
}

impl WorkingLevelStore for InMemoryStepBacktestingStore {
    type WorkingLevelProperties = BasicWLProperties;
    type CandleProperties = BasicCandleProperties;
    type OrderProperties = BasicOrderProperties;

    fn create_working_level(&mut self, properties: Self::WorkingLevelProperties) -> Result<WLId> {
        let id = xid::new().to_string();

        self.working_levels.insert(
            id.clone(),
            Item {
                id: id.clone(),
                props: properties,
            },
        );

        self.created_working_levels.insert(id.clone());

        Ok(id)
    }

    fn get_working_level_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Item<WLId, Self::WorkingLevelProperties>>> {
        Ok(self.working_levels.get(id).cloned())
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

        for order in self.get_working_level_chain_of_orders(id)? {
            self.orders.get_mut(&order.id).unwrap().props.main.status = OrderStatus::Closed;
        }

        self.removed_working_levels.insert(id.to_string());

        Ok(())
    }

    fn remove_working_level(&mut self, id: &str) -> Result<()> {
        if self.working_levels.remove(id).is_none() {
            bail!("a working level with an id {} doesn't exist", id);
        }

        self.working_level_big_corridors.remove(id);
        self.working_level_small_corridors.remove(id);

        self.working_level_max_crossing_values.remove(id);

        if let Some(orders) = self.working_level_chain_of_orders.remove(id) {
            for order in orders.iter() {
                self.remove_order(order)?;
            }
        }

        self.working_levels_with_moved_take_profits.remove(id);

        self.created_working_levels.remove(id);
        self.active_working_levels.remove(id);
        self.removed_working_levels.remove(id);

        Ok(())
    }

    fn get_created_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
        self.created_working_levels
            .iter()
            .map(|working_level_id| {
                self.get_working_level_by_id(working_level_id)?
                    .context(format!("no working level with an id {}", working_level_id))
            })
            .collect::<Result<_, _>>()
    }

    fn get_active_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
        self.active_working_levels
            .iter()
            .map(|working_level_id| {
                self.get_working_level_by_id(working_level_id)?
                    .context(format!("no working level with an id {}", working_level_id))
            })
            .collect::<Result<_, _>>()
    }

    fn get_removed_working_levels(&self) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
        self.removed_working_levels
            .iter()
            .map(|working_level_id| {
                self.get_working_level_by_id(working_level_id)?
                    .context(format!("no working level with an id {}", working_level_id))
            })
            .collect::<Result<_, _>>()
    }

    fn add_candle_to_working_level_corridor(
        &mut self,
        working_level_id: &str,
        candle_id: CandleId,
        corridor_type: CorridorType,
    ) -> Result<()> {
        if !self.working_levels.contains_key(working_level_id) {
            bail!(
                "a working level with an id {} doesn't exist",
                working_level_id
            );
        }

        if !self.candles.contains_key(&candle_id) {
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
    ) -> Result<Vec<Item<CandleId, Self::CandleProperties>>> {
        let candles = match corridor_type {
            CorridorType::Small => self.working_level_small_corridors.get(working_level_id),
            CorridorType::Big => self.working_level_big_corridors.get(working_level_id),
        };

        let candles = match candles {
            None => return Ok(Vec::new()),
            Some(candles) => candles
                .iter()
                .map(|candle_id| {
                    self.get_candle_by_id(candle_id)?
                        .context(format!("no candle with an id {}", candle_id))
                })
                .collect::<Result<Vec<_>, _>>(),
        }?;

        Ok(candles)
    }

    fn update_max_crossing_value_of_working_level(
        &mut self,
        working_level_id: &str,
        new_value: WLMaxCrossingValue,
    ) -> Result<()> {
        if !self.working_levels.contains_key(working_level_id) {
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
        if !self.working_levels.contains_key(working_level_id) {
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

    fn create_order(&mut self, properties: Self::OrderProperties) -> Result<OrderId> {
        let id = xid::new().to_string();

        let new_order = Item {
            id: id.clone(),
            props: properties,
        };

        self.orders.insert(id.clone(), new_order);

        Ok(id)
    }

    fn get_order_by_id(&self, id: &str) -> Result<Option<Item<OrderId, Self::OrderProperties>>> {
        Ok(self.orders.get(id).cloned())
    }

    fn remove_order(&mut self, id: &str) -> Result<()> {
        if self.orders.remove(id).is_none() {
            bail!("can't remove a non-existent order with an id {}", id);
        }

        Ok(())
    }

    fn add_order_to_working_level_chain_of_orders(
        &mut self,
        working_level_id: &str,
        order_id: OrderId,
    ) -> Result<()> {
        if !self.working_levels.contains_key(working_level_id) {
            bail!(
                "a working level with an id {} doesn't exist",
                working_level_id
            );
        }

        if !self.orders.contains_key(&order_id) {
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
    ) -> Result<Vec<Item<OrderId, Self::OrderProperties>>> {
        let orders = self.working_level_chain_of_orders.get(working_level_id);

        let orders = match orders {
            None => return Ok(Vec::new()),
            Some(orders) => orders
                .iter()
                .map(|order_id| {
                    self.get_order_by_id(order_id)?
                        .context(format!("no order with an id {}", order_id))
                })
                .collect::<Result<Vec<_>, _>>()?,
        };

        Ok(orders)
    }
}

impl InMemoryStepBacktestingStore {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_all_ticks(&self) -> Result<HashSet<TickId>> {
        Ok(self.ticks.keys().cloned().collect())
    }

    pub fn get_all_candles(&self) -> Result<HashSet<CandleId>> {
        Ok(self.candles.keys().cloned().collect())
    }

    pub fn get_all_angles(&self) -> Result<HashSet<AngleId>> {
        Ok(self.angles.keys().cloned().collect())
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

        self.ticks.retain(|tick_id, _| tick_is_in_use(tick_id));
    }

    /// For each candle checks whether it is in use. If a candle is not in use,
    /// removes it. We don't want candle list to grow endlessly.
    fn remove_unused_candles(&mut self) {
        let candle_in_angles = |candle_id: &CandleId| {
            self.angles
                .values()
                .any(|angle| &angle.props.candle_id == candle_id)
        };

        let candle_in_corridors = |candle_id: &CandleId| self.corridor_candles.contains(candle_id);
        let is_current_candle = |candle_id: &CandleId| {
            self.strategy_ticks_candles.current_candle.as_ref() == Some(candle_id)
        };
        let is_previous_candle = |candle_id: &CandleId| {
            self.strategy_ticks_candles.previous_candle.as_ref() == Some(candle_id)
        };

        self.candles.retain(|candle_id, _| {
            if candle_in_angles(candle_id)
                || candle_in_corridors(candle_id)
                || is_current_candle(candle_id)
                || is_previous_candle(candle_id)
            {
                return true;
            }

            false
        });
    }

    /// For each angle checks whether it is in use. If an angle is not in use,
    /// removes it. We don't want angle list to grow endlessly.
    fn remove_unused_angles(&mut self) {
        self.angles.retain(|angle_id, _| {
            [
                self.strategy_angles.max_angle.as_ref(),
                self.strategy_angles.min_angle.as_ref(),
                self.strategy_angles.virtual_max_angle.as_ref(),
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

    pub fn remove_unused_items(&mut self) -> Result<()> {
        // It's important to remove angles firstly. Otherwise it will block candles removal
        self.remove_unused_angles();
        self.remove_unused_candles();
        self.remove_unused_ticks();

        Ok(())
    }
}
