use crate::step::utils::entities::StrategySignals;
use crate::step::utils::stores::step_realtime_config_store::StepRealtimeConfigStore;
use crate::step::utils::stores::StepBacktestingStores;
use anyhow::Result;
use base::entities::candle::BasicCandleProperties;
use base::entities::BasicTickProperties;
use base::params::{StrategyCsvFileParams, StrategyParams};

use super::utils::entities::params::{StepPointParam, StepRatioParam};
use super::utils::level_utils::get_crossed_level;
use super::utils::orders::get_new_chain_of_orders;
use super::utils::stores::candle_store::CandleStore;
use super::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use super::utils::stores::tick_store::TickStore;
use super::utils::stores::working_level_store::WorkingLevelStore;
use super::utils::update_ticks;

/// Main iteration of the step strategy.
pub fn run_iteration(
    tick: BasicTickProperties,
    candle: Option<BasicCandleProperties>,
    signals: StrategySignals,
    stores: &mut StepBacktestingStores,
    params: &StrategyCsvFileParams<StepPointParam, StepRatioParam>,
) -> Result<()> {
    update_ticks(tick, &mut stores.main)?;

    let created_working_levels = stores.main.get_created_working_levels()?;

    let crossed_level = get_crossed_level(
        stores.main.get_current_tick()?.unwrap().props.bid,
        &created_working_levels,
    );

    if let Some(crossed_level) = crossed_level {
        if stores
            .main
            .get_working_level_chain_of_orders(&crossed_level.id)?
            .is_empty()
        {
            let chain_of_orders = get_new_chain_of_orders(
                crossed_level,
                params,
                stores.main.get_current_candle()?.unwrap().props.main.volatility,
                stores.config.balances.real
            )?;

            for order in chain_of_orders {
                let order_id = stores.main.create_order(order)?;
                stores.main.add_order_to_working_level_chain_of_orders(
                    &crossed_level.id,
                    order_id,
                )?;
            }

            stores.main.move_working_level_to_active(&crossed_level.id)?;
        }
    }



    Ok(())
}
