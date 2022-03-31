pub type OrderId = String;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrderType {
    Buy = 1,
    Sell = -1,
}

pub type OrderBasePrice = f32;

#[derive(Debug, PartialEq)]
pub struct OrderBasePrices {
    pub open_price: OrderBasePrice,
    pub stop_loss: OrderBasePrice,
    pub take_profit: OrderBasePrice,
}

pub type OrderVolume = f32;

#[derive(Debug, PartialEq)]
pub struct OrderBaseProperties {
    pub r#type: OrderType,
    pub prices: OrderBasePrices,
    pub volume: OrderVolume,
}
