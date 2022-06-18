use crate::step::utils::entities::StrategySignals;
use crate::step::utils::stores::step_realtime_config_store::StepRealtimeConfigStore;
use crate::step::utils::stores::step_realtime_store::StepRealtimeStore;
use crate::step::utils::stores::StepBacktestingStores;
use anyhow::Result;
use base::entities::candle::BasicCandle;
use base::entities::BasicTick;
use base::params::StrategyParams;

pub fn run_iteration(
    tick: &BasicTick,
    candle: Option<&BasicCandle>,
    signals: StrategySignals,
    stores: &mut StepBacktestingStores,
    params: &impl StrategyParams,
) -> Result<()> {
    todo!()
}
