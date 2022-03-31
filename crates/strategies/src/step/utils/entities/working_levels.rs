use base::entities::{OrderType, order::OrderId, candle::CandleId};
use chrono::NaiveDateTime;

pub type WLId = u32;
pub type WLPrice = f32;

pub struct WorkingLevelBaseProperties {
    pub price: WLPrice,
    pub r#type: OrderType,
    pub time: NaiveDateTime,
}

pub type WLMaxCrossingValue = f32;

pub struct WorkingLevelMaxCrossingValuesRow {
    pub working_level_id: WLId,
    pub value: WLMaxCrossingValue,
}

pub type WLIndex = u32;

pub struct WorkingLevelIndexesRow {
    pub working_level_id: WLId,
    pub index: WLIndex,
}

pub struct WorkingLevelCorridorsRow {
    pub candle_id: CandleId,
    pub working_level_id: WLId,
}

pub struct WorkingLevelChainOfOrdersRow {
    pub order_id: OrderId,
    pub working_level_id: WLId,
}
