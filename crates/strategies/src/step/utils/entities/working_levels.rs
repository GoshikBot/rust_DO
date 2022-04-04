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

pub type WLIndex = u32;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CorridorType {
    Small,
    Big,
}
