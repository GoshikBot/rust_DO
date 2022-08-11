use crate::step::utils::backtesting_charts::ChartIndex;
use base::entities::order::OrderType;
use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub type WLId = String;
pub type WLPrice = Decimal;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicWLProperties {
    pub price: WLPrice,
    pub r#type: OrderType,
    pub time: NaiveDateTime,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct BacktestingWLProperties {
    pub base: BasicWLProperties,
    pub chart_index: ChartIndex,
}

impl From<BacktestingWLProperties> for BasicWLProperties {
    fn from(properties: BacktestingWLProperties) -> Self {
        properties.base
    }
}

impl Default for BasicWLProperties {
    fn default() -> Self {
        Self {
            price: dec!(1.38),
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
