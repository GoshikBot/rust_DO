use base::entities::tick::{TickId, TickPrice};
use base::entities::BasicTick;
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct Tick {
    pub id: TickId,
    pub time: NaiveDateTime,
    pub ask: TickPrice,
    pub bid: TickPrice,
}
