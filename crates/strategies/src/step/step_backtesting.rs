use super::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::backtesting_charts::ChartTracesModifier;
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::entities::StrategySignals;
use crate::step::utils::helpers::Helpers;
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::level_utils::LevelUtils;
use crate::step::utils::order_utils::{
    OrderUtils, UpdateOrdersBacktestingStores, UpdateOrdersBacktestingUtils,
};
use crate::step::utils::stores::{StepBacktestingMainStore, StepBacktestingStores};
use crate::step::utils::StepBacktestingUtils;
use anyhow::Result;
use backtesting::trading_engine::TradingEngine;
use base::entities::candle::BasicCandleProperties;
use base::entities::BasicTickProperties;
use base::params::StrategyParams;

pub trait RunStepBacktestingIteration {
    /// Main iteration of the step strategy.
    fn run_iteration<T, H, U, N, R, D, E>(
        &self,
        tick: BasicTickProperties,
        candle: Option<StepBacktestingCandleProperties>,
        signals: StrategySignals,
        stores: &mut StepBacktestingStores<T>,
        utils: &StepBacktestingUtils<H, U, N, R, D, E>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<()>
    where
        T: StepBacktestingMainStore,

        H: Helpers,
        U: LevelUtils,
        N: LevelConditions,
        R: OrderUtils,
        D: ChartTracesModifier,
        E: TradingEngine;
}

#[derive(Default)]
pub struct StepBacktestingIterationRunner;

impl StepBacktestingIterationRunner {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RunStepBacktestingIteration for StepBacktestingIterationRunner {
    fn run_iteration<T, H, U, N, R, D, E>(
        &self,
        new_tick_props: BasicTickProperties,
        new_candle_props: Option<StepBacktestingCandleProperties>,
        signals: StrategySignals,
        stores: &mut StepBacktestingStores<T>,
        utils: &StepBacktestingUtils<H, U, N, R, D, E>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<()>
    where
        T: StepBacktestingMainStore,

        H: Helpers,
        U: LevelUtils,
        N: LevelConditions,
        R: OrderUtils,
        D: ChartTracesModifier,
        E: TradingEngine,
    {
        let current_tick = stores.main.create_tick(new_tick_props)?;

        if let Some(current_tick) = stores.main.get_current_tick()? {
            stores.main.update_previous_tick(current_tick.id)?;
        }

        stores.main.update_current_tick(current_tick.id)?;

        let (current_candle, new_candle_appeared) = match new_candle_props {
            Some(candle_props) => {
                let current_candle = stores.main.create_candle(candle_props)?;

                if let Some(current_candle) = stores.main.get_current_candle()? {
                    stores.main.update_previous_candle(current_candle.id)?;
                }

                stores
                    .main
                    .update_current_candle(current_candle.id.clone())?;

                (Some(current_candle), true)
            }
            None => (stores.main.get_current_candle()?, false),
        };

        let created_working_levels = stores.main.get_created_working_levels()?;

        let crossed_level = utils
            .level_utils
            .get_crossed_level(current_tick.props.bid, &created_working_levels);

        if let Some(crossed_level) = crossed_level {
            if stores
                .main
                .get_working_level_chain_of_orders(&crossed_level.id)?
                .is_empty()
            {
                let chain_of_orders = utils.order_utils.get_new_chain_of_orders(
                    crossed_level,
                    params,
                    stores
                        .main
                        .get_current_candle()?
                        .unwrap()
                        .props
                        .base
                        .volatility,
                    stores.config.trading_engine.balances.real,
                )?;

                for order_props in chain_of_orders {
                    let order = stores.main.create_order(order_props)?;
                    stores
                        .main
                        .add_order_to_working_level_chain_of_orders(&crossed_level.id, order.id)?;
                }

                stores
                    .main
                    .move_working_level_to_active(&crossed_level.id)?;
            }
        }

        utils
            .level_utils
            .remove_active_working_levels_with_closed_orders(&mut stores.main)?;

        if let Some(current_candle) = current_candle {
            utils.order_utils.update_orders_backtesting(
                &current_tick.props,
                &current_candle.props,
                params,
                UpdateOrdersBacktestingStores {
                    main: &mut stores.main,
                    config: &mut stores.config,
                    statistics: &mut stores.statistics,
                },
                UpdateOrdersBacktestingUtils {
                    trading_engine: &utils.trading_engine,
                    chart_traces_modifier: &utils.chart_traces_modifier,
                    level_conditions: &utils.level_conditions,
                },
                signals.no_trading_mode,
            )?;
        }

        utils
            .level_utils
            .update_max_crossing_value_of_active_levels(&mut stores.main, current_tick.props.bid)?;

        Ok(())
    }
}
