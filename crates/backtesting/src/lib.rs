use polars_lazy::prelude::LazyFrame;

#[derive(Debug, PartialEq, Eq)]
pub enum BacktestingOrderStatus {
    Pending = 0,
    Opened = 1,
    Closed = -1,
}

pub type CurrentPrice = f32;

#[derive(Debug)]
pub enum PlaceOrderBy {
    OpenPrice,
    CurrentPrice(CurrentPrice),
}

#[derive(Debug)]
pub enum ClosePositionBy {
    TakeProfit,
    StopLoss,
    CurrentPrice(CurrentPrice),
}

#[derive(Debug)]
pub enum Mode {
    Optimization,
    Debug,
}

pub type DFDate = &'static str;
pub type DFMarginFromStart = u32;

pub struct DataFrameConstraints {
    pub start: DFDate,
    pub end: DFDate,
    pub margin_from_start: DFMarginFromStart,
}

pub type Balance = f32;
pub type Units = i32;
pub type Trades = u32;

pub struct BacktestingLowLevelData {
    pub initial_balance: Balance,
    pub processing_balance: Balance,
    pub real_balance: Balance,
    pub units: Units,
    pub trades: Trades,
}

impl Default for BacktestingLowLevelData {
    fn default() -> Self {
        Self {
            initial_balance: 10_000.0,
            processing_balance: 10_000.0,
            real_balance: 10_000.0,
            units: 0,
            trades: 0,
        }
    }
}

pub type Leverage = f32;
pub type Spread = f32;

pub struct BacktestingConfig {
    pub leverage: Leverage,
    pub use_spread: bool,
    pub spread: Spread,
}

impl Default for BacktestingConfig {
    fn default() -> Self {
        Self {
            leverage: 0.01,
            use_spread: true,
            spread: 0.00010,
        }
    }
}

pub struct DataFrames {
    pub ticks_tf: LazyFrame,
    pub candles_tf: LazyFrame,
}
