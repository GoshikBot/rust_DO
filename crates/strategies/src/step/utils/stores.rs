use crate::step::utils::entities::angle::AngleId;
use crate::step::utils::entities::Diff;
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use crate::step::utils::stores::step_realtime_config_store::StepRealtimeConfigStore;
use base::entities::{candle::CandleId, tick::TickId, Level, MovementType};

pub mod angle_store;
pub mod candle_store;
pub mod in_memory_step_backtesting_store;
pub mod in_memory_step_realtime_config_store;
pub mod step_realtime_config_store;
pub mod tick_store;
pub mod working_level_store;

const DEFAULT_INITIAL_BALANCE_BACKTESTING: StepBacktestingBalance = 10_000.0;
const DEFAULT_LEVERAGE_BACKTESTING: Leverage = 0.01;
const DEFAULT_SPREAD_BACKTESTING: Spread = 0.00010;

#[derive(Default)]
pub struct StepBacktestingStores {
    pub main: InMemoryStepBacktestingStore,
    pub config: StepBacktestingConfig,
    pub statistics: StepBacktestingStatistics,
}

pub type SettingFile = &'static str;
pub type Symbol = &'static str;

pub type BacktestingIndex = u32;

#[derive(Default)]
pub struct StepStrategyAngles {
    pub angle_of_second_level_after_bargaining_tendency_change: Option<AngleId>,
    pub tendency_change_angle: Option<AngleId>,
    pub min_angle: Option<AngleId>,
    pub virtual_min_angle: Option<AngleId>,
    pub max_angle: Option<AngleId>,
    pub virtual_max_angle: Option<AngleId>,
    pub min_angle_before_bargaining_corridor: Option<AngleId>,
    pub max_angle_before_bargaining_corridor: Option<AngleId>,
}

#[derive(Default)]
pub struct StepStrategyTicksCandles {
    pub current_tick: Option<TickId>,
    pub previous_tick: Option<TickId>,

    pub current_candle: Option<CandleId>,
    pub previous_candle: Option<CandleId>,
}

#[derive(Default)]
pub struct StepDiffs {
    pub current: Option<Diff>,
    pub previous: Option<Diff>,
}

pub type StepBacktestingBalance = f32;

pub struct StepBacktestingBalances {
    pub initial: StepBacktestingBalance,
    pub processing: StepBacktestingBalance,
    pub real: StepBacktestingBalance,
}

impl Default for StepBacktestingBalances {
    fn default() -> Self {
        StepBacktestingBalances {
            initial: DEFAULT_INITIAL_BALANCE_BACKTESTING,
            processing: DEFAULT_INITIAL_BALANCE_BACKTESTING,
            real: DEFAULT_INITIAL_BALANCE_BACKTESTING,
        }
    }
}

pub type Units = i32;
pub type Trades = i32;

pub type Leverage = f32;
pub type Spread = f32;

pub struct StepBacktestingConfig {
    pub tendency: MovementType,
    pub tendency_changed_on_crossing_bargaining_corridor: bool,
    pub second_level_after_bargaining_tendency_change_is_created: bool,
    pub skip_creating_new_working_level: bool,
    pub diffs: StepDiffs,
    pub balances: StepBacktestingBalances,
    pub units: Units,
    pub trades: Trades,
    pub leverage: Leverage,
    pub spread: Spread,
    pub use_spread: bool,
    pub indexes: StepBacktestingIndexes,
}

impl Default for StepBacktestingConfig {
    fn default() -> Self {
        StepBacktestingConfig {
            tendency: Default::default(),
            tendency_changed_on_crossing_bargaining_corridor: false,
            second_level_after_bargaining_tendency_change_is_created: false,
            skip_creating_new_working_level: false,
            diffs: Default::default(),
            balances: Default::default(),
            units: 0,
            trades: 0,
            leverage: DEFAULT_LEVERAGE_BACKTESTING,
            spread: DEFAULT_SPREAD_BACKTESTING,
            use_spread: true,
            indexes: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct StepBacktestingIndexes {
    pub working_level_index: BacktestingIndex,
    pub stop_loss_index: BacktestingIndex,
    pub take_profit_index: BacktestingIndex,
    pub tf_entity_index: BacktestingIndex,
}

pub type BacktestingStatisticNumber = u32;

#[derive(Default)]
pub struct StepBacktestingStatistics {
    pub number_of_working_levels: BacktestingStatisticNumber,
    pub number_of_tendency_changes: BacktestingStatisticNumber,

    pub deleted_by_being_close_to_another_one: BacktestingStatisticNumber,
    pub deleted_by_another_active_chain_of_orders: BacktestingStatisticNumber,
    pub deleted_by_expiration_by_distance: BacktestingStatisticNumber,
    pub deleted_by_expiration_by_time: BacktestingStatisticNumber,
    pub deleted_by_price_being_beyond_stop_loss: BacktestingStatisticNumber,
    pub deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing:
        BacktestingStatisticNumber,
    pub deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing:
        BacktestingStatisticNumber,
    pub deleted_by_exceeding_activation_crossing_distance: BacktestingStatisticNumber,
}
