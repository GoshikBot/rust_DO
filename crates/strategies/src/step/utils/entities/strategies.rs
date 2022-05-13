use base::entities::{candle::CandleId, tick::TickId, Level, MovementType};

use super::{angle::AngleId, Diff};

pub type SettingFile = &'static str;
pub type Symbol = &'static str;

#[derive(Default)]
pub struct StrategyAngles {
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
pub struct StrategyDiffs {
    pub current_diff: Option<Diff>,
    pub previous_diff: Option<Diff>,
}

#[derive(Default)]
pub struct StrategyTicksCandles {
    pub current_tick: Option<TickId>,
    pub previous_tick: Option<TickId>,

    pub current_candle: Option<CandleId>,
    pub previous_candle: Option<CandleId>,
}

pub type BacktestingIndex = u32;

#[derive(Default)]
pub struct BacktestingIndexes {
    pub working_level_index: BacktestingIndex,
    pub stop_loss_index: BacktestingIndex,
    pub take_profit_index: BacktestingIndex,
    pub tf_entity_index: BacktestingIndex,
}

pub type BacktestingStatisticNumber = u32;

#[derive(Default)]
pub struct BacktestingStatistics {
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
