use std::collections::HashMap;

use backtesting::{BacktestingLowLevelData, BacktestingConfig, Balance, Units, Trades, Leverage, Spread, backtesting_base_store::BacktestingBaseStore};
use base::entities::{CandleBaseProperties, candle::{CandleEdgePrice, CandleId}, MovementType, TickBaseProperties, CandleEdgePrices, tick::TickId, OrderBaseProperties, order::OrderId, Level};
use simple_error::SimpleResult;

use crate::step::utils::entities::{settings::{SettingProgramName, SettingTableName, RatioSettingValue, PointSettingValue}, strategies::{StrategyBaseConfig, StrategyAngles, StrategyDiffs, StrategyTicksCandles, Symbol, BacktestingIndexes, BacktestingStatistics, BacktestingIndex, BacktestingStatisticNumber}, angles::{AngleId, Angle, AngleRow}, Diff, working_levels::{WLId, WorkingLevelBasePropertiesRow, WorkingLevelMaxCrossingValuesRow, WorkingLevelSmallCorridorElement, WorkingLevelBigCorridorElement, WorkingLevelChainOfOrdersRow}};

use super::base::{StepBaseStore, StepBacktestingStore};

pub struct InMemoryStepBacktestingStore {
    candle_base_properties: HashMap<CandleId, CandleBaseProperties>,
    candle_edge_prices: HashMap<CandleId, CandleEdgePrices>,

    tick_base_properties: HashMap<TickId, TickBaseProperties>,

    angles: HashMap<AngleId, Angle>,

    working_level_base_properties: HashMap<WLId, WorkingLevelBasePropertiesRow>,
    working_level_max_crossing_values: HashMap<WLId, WorkingLevelMaxCrossingValuesRow>,
    working_level_small_corridors: Vec<WorkingLevelSmallCorridorElement>,
    working_level_big_corridors: Vec<WorkingLevelBigCorridorElement>,

    working_level_chain_of_orders: Vec<WorkingLevelChainOfOrdersRow>,

    order_base_prices: HashMap<OrderId, OrderBaseProperties>,
    order_base_properties: HashMap<OrderId, OrderBaseProperties>,

    backtesting_limit_orders: Vec<OrderId>,

    setting_names: HashMap<SettingProgramName, SettingTableName>,
    ratio_settings: HashMap<SettingProgramName, RatioSettingValue>,
    point_settings: HashMap<SettingProgramName, PointSettingValue>,

    strategy_base_config: StrategyBaseConfig,
    strategy_angles: StrategyAngles,
    strategy_diffs: StrategyDiffs,
    strategy_ticks_candles: StrategyTicksCandles,

    backtesting_indexes: BacktestingIndexes,
    backtesting_statistics: BacktestingStatistics,

    backtesting_low_level_data: BacktestingLowLevelData,
    backtesting_config: BacktestingConfig,
}

impl StepBaseStore for InMemoryStepBacktestingStore {
    fn get_symbol(&self) -> SimpleResult<Symbol> {
        Ok(self.strategy_base_config.symbol)
    }

    fn get_tendency(&self) -> SimpleResult<MovementType> {
        Ok(self.strategy_base_config.tendency)
    }

    fn update_tendency(&mut self, value: MovementType) -> SimpleResult<()> {
        self.strategy_base_config.tendency = value;
        Ok(())
    }

    fn get_tendency_changed_on_crossing_bargaining_corridor(&self) -> SimpleResult<bool> {
        Ok(self.strategy_base_config.tendency_changed_on_crossing_bargaining_corridor)
    }

    fn update_tendency_changed_on_crossing_bargaining_corridor(&mut self, value: bool) -> SimpleResult<()> {
        self.strategy_base_config.tendency_changed_on_crossing_bargaining_corridor = value;
        Ok(())
    }

    fn get_second_level_after_bargaining_tendency_change_is_created(&self) -> SimpleResult<bool> {
        Ok(self.strategy_base_config.second_level_after_bargaining_tendency_change_is_created)
    }

    fn update_second_level_after_bargaining_tendency_change_is_created(&mut self, value: bool) -> SimpleResult<()> {
        self.strategy_base_config.second_level_after_bargaining_tendency_change_is_created = value;
        Ok(())
    }

    fn get_skip_creating_new_working_level(&self) -> SimpleResult<bool> {
        Ok(self.strategy_base_config.skip_creating_new_working_level)
    }

    fn update_skip_creating_new_working_level(&mut self, value: bool) -> SimpleResult<()> {
        self.strategy_base_config.skip_creating_new_working_level = value;
        Ok(())
    }

    fn get_no_trading_mode(&self) -> SimpleResult<bool> {
        Ok(self.strategy_base_config.no_trading_mode)
    }

    fn update_no_trading_mode(&mut self, value: bool) -> SimpleResult<()> {
        self.strategy_base_config.no_trading_mode = value;
        Ok(())
    }

    fn create_angle(&mut self, id: AngleId, new_angle: Angle) -> SimpleResult<()> {
        let new_angle_row = AngleRow { 
            id,
            candle_id: new_angle.candle_id,
            r#type: new_angle.r#type
        };

        self.angles.insert(id, v)

        Ok(())
    }

}

