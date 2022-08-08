use std::collections::HashSet;

use anyhow::Result;
use base::entities::{tick::TickId, Item};
use base::stores::tick_store::BasicTickStore;

pub trait StepTickStore: BasicTickStore {
    fn get_current_tick(&self) -> Result<Option<Item<TickId, Self::TickProperties>>>;
    fn update_current_tick(&mut self, tick_id: TickId) -> Result<()>;

    fn get_previous_tick(&self) -> Result<Option<Item<TickId, Self::TickProperties>>>;
    fn update_previous_tick(&mut self, tick_id: TickId) -> Result<()>;
}
