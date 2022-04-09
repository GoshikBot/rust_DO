use base::entities::{candle::CandleId, order::OrderId, OrderType};
use chrono::{NaiveDateTime, Utc};

pub type WLId = String;
pub type WLPrice = f32;

#[derive(Debug, Clone, PartialEq)]
pub struct WorkingLevelBaseProperties {
    pub price: WLPrice,
    pub r#type: OrderType,
    pub time: NaiveDateTime,
}

impl Default for WorkingLevelBaseProperties {
    fn default() -> Self {
        Self {
            price: 1.38,
            r#type: OrderType::Buy,
            time: Utc::now().naive_utc(),
        }
    }
}

pub type WLMaxCrossingValue = f32;

pub type WLIndex = u32;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CorridorType {
    Small,
    Big,
}
