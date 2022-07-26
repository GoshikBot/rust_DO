use std::collections::HashSet;

use anyhow::Result;
use base::entities::{tick::TickId, Item};

pub trait TickStore {
    type TickProperties;

    fn create_tick(&mut self, properties: Self::TickProperties) -> Result<TickId>;
    fn get_tick_by_id(&self, tick_id: &str) -> Result<Option<Item<TickId, Self::TickProperties>>>;

    fn get_current_tick(&self) -> Result<Option<Item<TickId, Self::TickProperties>>>;
    fn update_current_tick(&mut self, tick_id: TickId) -> Result<()>;

    fn get_previous_tick(&self) -> Result<Option<Item<TickId, Self::TickProperties>>>;
    fn update_previous_tick(&mut self, tick_id: TickId) -> Result<()>;
}
