pub mod candle;
pub mod tick;

pub use candle::{CandleBaseProperties, CandleEdgePrices, CandleType};
pub use tick::TickBaseProperties;

pub const LOT: i32 = 100_000;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Level {
    Min = -1,
    Max = 1,
}

#[derive(Debug, Copy, Clone)]
pub enum MovementType {
    Unknown = 0,
    Up = 1,
    Down = -1,
}

impl Default for MovementType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Edge {
    High = 1,
    Low = -1,
}
