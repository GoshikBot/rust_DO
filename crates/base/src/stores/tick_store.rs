use crate::entities::tick::TickId;
use crate::entities::Item;
use anyhow::Result;

pub trait BasicTickStore {
    type TickProperties;

    fn create_tick(&mut self, properties: Self::TickProperties) -> Result<Item<TickId, Self::TickProperties>>;
    fn get_tick_by_id(&self, tick_id: &str) -> Result<Option<Item<TickId, Self::TickProperties>>>;
}
