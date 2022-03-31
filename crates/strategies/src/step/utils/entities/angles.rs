use base::entities::{Level, candle::CandleId};

pub type AngleId = String;

#[derive(Debug, Clone)]
pub struct Angle {
    pub candle_id: CandleId,
    pub r#type: Level,
}
