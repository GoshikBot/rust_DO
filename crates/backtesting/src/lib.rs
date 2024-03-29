use base::entities::candle::BasicCandleProperties;
use base::entities::tick::{TickPrice, TickTime};
use base::entities::{BasicTickProperties, StrategyTimeframes};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub mod historical_data;
pub mod trading_engine;

const DEFAULT_INITIAL_BALANCE_BACKTESTING: Balance = dec!(10_000);
const DEFAULT_LEVERAGE_BACKTESTING: Leverage = dec!(0.01);
const DEFAULT_SPREAD_BACKTESTING: Spread = dec!(0.00010);

const TIME_PATTERN_FOR_PATH: &str = "%Y-%m-%d_%H-%M";

#[derive(Debug)]
pub enum OpenPositionBy {
    OpenPrice,
    CurrentTickPrice(TickPrice),
}

#[derive(Debug)]
pub enum ClosePositionBy {
    TakeProfit,
    StopLoss,
    CurrentTickPrice(TickPrice),
}

pub type Balance = Decimal;

#[derive(Debug)]
pub struct BacktestingBalances {
    pub initial: Balance,
    pub processing: Balance,
    pub real: Balance,
}

impl BacktestingBalances {
    pub fn new(initial_balance: Balance) -> Self {
        Self {
            initial: initial_balance,
            processing: initial_balance,
            real: initial_balance,
        }
    }
}

impl Default for BacktestingBalances {
    fn default() -> Self {
        Self {
            initial: DEFAULT_INITIAL_BALANCE_BACKTESTING,
            processing: DEFAULT_INITIAL_BALANCE_BACKTESTING,
            real: DEFAULT_INITIAL_BALANCE_BACKTESTING,
        }
    }
}

pub type Units = i32;
pub type Trades = i32;

pub type Leverage = Decimal;
pub type Spread = Decimal;

#[derive(Debug)]
pub struct BacktestingTradingEngineConfig {
    pub balances: BacktestingBalances,
    pub units: Units,
    pub trades: Trades,
    pub leverage: Leverage,
    pub spread: Spread,
    pub use_spread: bool,
}

impl Default for BacktestingTradingEngineConfig {
    fn default() -> Self {
        Self {
            balances: BacktestingBalances::default(),
            units: 0,
            trades: 0,
            leverage: DEFAULT_LEVERAGE_BACKTESTING,
            spread: DEFAULT_SPREAD_BACKTESTING,
            use_spread: true,
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct HistoricalData<C, T> {
    pub candles: Vec<Option<C>>,
    pub ticks: Vec<Option<T>>,
}

#[derive(Debug)]
pub struct StrategyInitConfig {
    pub symbol: String,
    pub timeframes: StrategyTimeframes,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
}

pub fn get_path_name_for_data_config(strategy_config: &StrategyInitConfig) -> String {
    let StrategyInitConfig {
        symbol,
        timeframes:
            StrategyTimeframes {
                candle: candle_timeframe,
                tick: tick_timeframe,
            },
        end_time,
        duration,
    } = strategy_config;

    format!(
        "{}_{}_{}_{}_{}_({}_weeks)",
        symbol,
        candle_timeframe,
        tick_timeframe,
        end_time.format(TIME_PATTERN_FOR_PATH),
        duration.num_minutes(),
        duration.num_weeks()
    )
}
