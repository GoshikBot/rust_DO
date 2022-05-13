use crate::step::utils::entities::order::OrderType;
use chrono::{NaiveDateTime, Utc};

pub type WLId = String;
pub type WLPrice = f32;

#[derive(Debug, Clone, PartialEq)]
pub struct WorkingLevel {
    pub id: WLId,
    pub price: WLPrice,
    pub r#type: OrderType,
    pub time: NaiveDateTime,
}

impl Default for WorkingLevel {
    fn default() -> Self {
        Self {
            id: String::from("1"),
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
