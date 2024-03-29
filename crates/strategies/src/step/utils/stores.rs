use crate::step::utils::backtesting_charts::{AmountOfCandles, StepBacktestingChartTraces};
use crate::step::utils::entities::angle::{AngleId, BasicAngleProperties};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::entities::order::StepOrderProperties;
use crate::step::utils::entities::working_levels::BacktestingWLProperties;
use crate::step::utils::entities::Diff;
use crate::step::utils::stores::angle_store::StepAngleStore;
use crate::step::utils::stores::candle_store::StepCandleStore;
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use crate::step::utils::stores::tick_store::StepTickStore;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use backtesting::BacktestingTradingEngineConfig;
use base::entities::tick::HistoricalTickPrice;
use base::entities::{candle::CandleId, tick::TickId, BasicTickProperties, Tendency};
use base::stores::candle_store::BasicCandleStore;
use base::stores::order_store::BasicOrderStore;
use base::stores::tick_store::BasicTickStore;

pub mod angle_store;
pub mod candle_store;
pub mod in_memory_step_backtesting_store;
pub mod tick_store;
pub mod working_level_store;

pub struct StepBacktestingStores<T>
where
    T: StepBacktestingMainStore,
{
    pub main: T,
    pub config: StepBacktestingConfig,
    pub statistics: StepBacktestingStatistics,
}

pub trait StepBacktestingMainStore:
    StepTickStore<TickProperties = BasicTickProperties<HistoricalTickPrice>>
    + StepCandleStore<CandleProperties = StepBacktestingCandleProperties>
    + StepAngleStore<
        AngleProperties = BasicAngleProperties,
        CandleProperties = StepBacktestingCandleProperties,
    > + StepWorkingLevelStore<
        WorkingLevelProperties = BacktestingWLProperties,
        CandleProperties = StepBacktestingCandleProperties,
        OrderProperties = StepOrderProperties,
    > + BasicOrderStore<OrderProperties = StepOrderProperties>
{
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

#[derive(Debug, Default, Copy, Clone)]
pub struct StepDiffs {
    pub current: Option<Diff>,
    pub previous: Option<Diff>,
}

#[derive(Debug, Default)]
pub struct StepConfig {
    pub tendency: Tendency,
    pub tendency_changed_on_crossing_bargaining_corridor: bool,
    pub second_level_after_bargaining_tendency_change_is_created: bool,
    pub skip_creating_new_working_level: bool,
    pub diffs: StepDiffs,
}

#[derive(Debug)]
pub struct StepBacktestingConfig {
    pub base: StepConfig,
    pub trading_engine: BacktestingTradingEngineConfig,
    pub chart_traces: StepBacktestingChartTraces,
}

impl StepBacktestingConfig {
    pub fn default(total_amount_of_candles: AmountOfCandles) -> Self {
        Self {
            base: Default::default(),
            trading_engine: Default::default(),
            chart_traces: StepBacktestingChartTraces::new(total_amount_of_candles),
        }
    }
}

pub type BacktestingStatisticNumber = u32;

#[derive(Debug, Default)]
pub struct StepBacktestingStatistics {
    pub number_of_working_levels: BacktestingStatisticNumber,
    pub number_of_tendency_changes: BacktestingStatisticNumber,

    pub deleted_by_being_close_to_another_one: BacktestingStatisticNumber,
    pub deleted_by_expiration_by_distance: BacktestingStatisticNumber,
    pub deleted_by_expiration_by_time: BacktestingStatisticNumber,
    pub deleted_by_price_being_beyond_stop_loss: BacktestingStatisticNumber,
    pub deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing:
        BacktestingStatisticNumber,
    pub deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing:
        BacktestingStatisticNumber,
    pub deleted_by_exceeding_activation_crossing_distance: BacktestingStatisticNumber,
}
