pub mod angles;
pub mod strategies;
pub mod working_levels;

#[derive(Debug, Clone, Copy)]
pub enum Diff {
    Greater = 1,
    Less = -1,
}
