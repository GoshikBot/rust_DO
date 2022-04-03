pub mod candle;
pub mod order;
pub mod tick;

pub use candle::{CandleBaseProperties, CandleEdgePrices, CandleType};
pub use order::{OrderBasePrices, OrderBaseProperties, OrderType};
pub use tick::TickBaseProperties;

const LOT: i32 = 100_000;

#[derive(Debug)]
pub enum Param {
    Ratio(String),
    Point(f32),
}

#[derive(Debug, Clone, Copy)]
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
