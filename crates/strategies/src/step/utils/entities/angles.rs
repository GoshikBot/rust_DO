use base::entities::{candle::CandleId, Level};

pub type AngleId = String;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AngleBaseProperties {
    pub candle_id: CandleId,
    pub r#type: Level,
}

impl Default for AngleBaseProperties {
    fn default() -> Self {
        Self {
            candle_id: String::from("1"),
            r#type: Level::Min,
        }
    }
}
