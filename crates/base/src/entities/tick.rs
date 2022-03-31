use chrono::NaiveDateTime;

pub type TickPrice = f32;
pub type TickId = String;

#[derive(Debug, PartialEq, Clone)]
pub struct TickBaseProperties {
    time: NaiveDateTime,
    ask: TickPrice,
    bid: TickPrice,
}
