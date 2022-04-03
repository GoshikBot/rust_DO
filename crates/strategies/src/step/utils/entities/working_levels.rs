use base::entities::{candle::CandleId, order::OrderId, OrderType};
use chrono::NaiveDateTime;

pub type WLId = String;
pub type WLPrice = f32;

pub struct WorkingLevelBaseProperties {
    pub price: WLPrice,
    pub r#type: OrderType,
    pub time: NaiveDateTime,
}

pub type WLMaxCrossingValue = f32;

pub struct WorkingLevelMaxCrossingValue {
    pub working_level_id: WLId,
    pub value: WLMaxCrossingValue,
}

pub type WLIndex = u32;

pub struct WorkingLevelIndex {
    pub working_level_id: WLId,
    pub index: WLIndex,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CorridorType {
    Small,
    Big,
}

#[derive(Eq, PartialEq)]
pub struct WorkingLevelCorridorCandle {
    pub candle_id: CandleId,
    pub working_level_id: WLId,
}

pub struct WorkingLevelOrder {
    pub order_id: OrderId,
    pub working_level_id: WLId,
}
