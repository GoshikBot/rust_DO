use crate::entities::order::{OrderId, OrderStatus};
use crate::entities::Item;
use anyhow::Result;

pub trait BasicOrderStore {
    type OrderProperties;

    fn create_order(&mut self, properties: Self::OrderProperties) -> Result<Item<OrderId, Self::OrderProperties>>;
    fn get_order_by_id(&self, id: &str) -> Result<Option<Item<OrderId, Self::OrderProperties>>>;
    fn get_all_orders(&self) -> Result<Vec<Item<OrderId, Self::OrderProperties>>>;
    fn update_order_status(&mut self, order_id: &str, new_status: OrderStatus) -> Result<()>;
}
