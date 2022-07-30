use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::step::utils::entities::working_levels::WLId;

pub type OrderId = String;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrderType {
    Buy = 1,
    Sell = -1,
}

pub type OrderPrice = Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct BasicOrderPrices {
    pub open: OrderPrice,
    pub stop_loss: OrderPrice,
    pub take_profit: OrderPrice,
}

impl Default for BasicOrderPrices {
    fn default() -> Self {
        Self {
            open: dec!(1.38),
            stop_loss: dec!(1.37),
            take_profit: dec!(1.39),
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

pub type OrderVolume = Decimal;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BasicOrderProperties {
    pub main: BasicOrderMainProperties,
    pub prices: BasicOrderPrices,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicOrderMainProperties {
    pub r#type: OrderType,
    pub volume: OrderVolume,
    pub status: OrderStatus,
    pub working_level_id: WLId,
}

impl Default for BasicOrderMainProperties {
    fn default() -> Self {
        Self {
            r#type: OrderType::Buy,
            volume: dec!(0),
            status: Default::default(),
            working_level_id: String::from("1"),
        }
    }
}