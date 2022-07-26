use crate::step::utils::entities::working_levels::WLId;

pub type OrderId = String;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrderType {
    Buy = 1,
    Sell = -1,
}

pub type OrderBasePrice = f32;

#[derive(Debug, PartialEq, Clone)]
pub struct OrderPrices {
    pub open: OrderBasePrice,
    pub stop_loss: OrderBasePrice,
    pub take_profit: OrderBasePrice,
}

impl Default for OrderPrices {
    fn default() -> Self {
        Self {
            open: 1.38,
            stop_loss: 1.37,
            take_profit: 1.39,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum OrderStatus {
    Pending = 0,
    Opened = 1,
    Closed = -1,
}

impl Default for OrderStatus {
    fn default() -> Self {
        Self::Pending
    }
}

pub type OrderVolume = f32;

#[derive(Debug, Clone, Default)]
pub struct BasicOrderProperties {
    pub main: OrderMainProperties,
    pub prices: OrderPrices,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrderMainProperties {
    pub r#type: OrderType,
    pub volume: OrderVolume,
    pub status: OrderStatus,
    pub working_level_id: WLId,
}

impl Default for OrderMainProperties {
    fn default() -> Self {
        Self {
            r#type: OrderType::Buy,
            volume: 0.0,
            status: Default::default(),
            working_level_id: String::from("1"),
        }
    }
}
