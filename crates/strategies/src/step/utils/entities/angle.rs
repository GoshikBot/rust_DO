use base::entities::{candle::CandleId, Item, Level};

pub type AngleId = String;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AngleState {
    Real,
    Virtual,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BasicAngleProperties {
    pub r#type: Level,
    pub state: AngleState,
}

impl AsRef<BasicAngleProperties> for BasicAngleProperties {
    fn as_ref(&self) -> &BasicAngleProperties {
        self
    }
}

impl Default for BasicAngleProperties {
    fn default() -> Self {
        Self {
            r#type: Level::Min,
            state: AngleState::Real,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FullAngleProperties<P, C> {
    pub base: P,
    pub candle: Item<CandleId, C>,
}
