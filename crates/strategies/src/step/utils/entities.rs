pub mod angle;
pub mod candle;
pub mod order;
pub mod strategies;
pub mod tick;
pub mod working_levels;

#[derive(Debug, Clone, Copy)]
pub enum Diff {
    Greater = 1,
    Less = -1,
}
