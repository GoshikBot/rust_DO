use base::entities::{candle::CandleId, Level};

pub type AngleId = String;

#[derive(Debug, Clone)]
pub struct Angle {
    pub candle_id: CandleId,
    pub r#type: Level,
}
