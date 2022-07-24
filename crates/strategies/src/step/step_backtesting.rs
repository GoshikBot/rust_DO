use crate::step::utils::entities::StrategySignals;
use crate::step::utils::stores::step_realtime_config_store::StepRealtimeConfigStore;
use crate::step::utils::stores::step_realtime_store::StepRealtimeStore;
use crate::step::utils::stores::StepBacktestingStores;
use anyhow::Result;
use base::entities::candle::BasicCandle;
use base::entities::BasicTick;
use base::params::{StrategyParams, StrategyCsvFileParams};

use super::utils::entities::params::{StepPointParam, StepRatioParam};
use super::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;

pub fn run_iteration(
    tick: &BasicTick,
    candle: Option<&BasicCandle>,
    signals: StrategySignals,
    stores: &mut StepBacktestingStores,
    params: &StrategyCsvFileParams<StepPointParam, StepRatioParam>
) -> Result<()> {
    // let 
    // stores.main.create_tick(id, tick_base_properties)
    todo!()
}

fn update_ticks(new_tick: &BasicTick, store: &mut InMemoryStepBacktestingStore) {
    // let 

    // if store.get_current_tick().is_none() {
    //     store.upda
    // }
}
