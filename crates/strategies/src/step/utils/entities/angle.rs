use base::entities::{candle::CandleId, Item, Level};

pub type AngleId = String;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicAngleProperties {
    pub r#type: Level,
}

impl Default for BasicAngleProperties {
    fn default() -> Self {
        Self { r#type: Level::Min }
    }
}

pub struct FullAngleProperties<P, C> {
    pub base: P,
    pub candle: Item<CandleId, C>,
}
