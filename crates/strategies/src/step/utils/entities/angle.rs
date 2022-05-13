use base::entities::{candle::CandleId, Level};

pub type AngleId = String;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Angle {
    pub id: AngleId,
    pub candle_id: CandleId,
    pub r#type: Level,
}

impl Default for Angle {
    fn default() -> Self {
        Self {
            id: String::from("1"),
            candle_id: String::from("1"),
            r#type: Level::Min,
        }
    }
}
