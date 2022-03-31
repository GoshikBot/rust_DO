use crate::entities::{order::OrderId, OrderBasePrices, OrderBaseProperties};

pub trait OrderBaseStore {
    fn get_order_base_prices(order_id: OrderId) -> OrderBasePrices;
    fn get_order_base_properties(order_id: OrderId) -> OrderBaseProperties;

    fn add_order(base_prices: OrderBasePrices, base_properties: OrderBaseProperties) -> OrderId;
    fn remove_order(order_id: OrderId);
}