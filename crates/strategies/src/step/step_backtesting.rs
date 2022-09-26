use super::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::angle_utils::{AngleUtils, ExistingDiffs};
use crate::step::utils::backtesting_charts::{
    ChartIndex, ChartTraceEntity, StepBacktestingChartTraces,
};
use crate::step::utils::corridors::{
    Corridors, UpdateCorridorsNearWorkingLevelsUtils, UpdateSmallCorridorNearLevelUtils,
};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::entities::{
    Diff, FakeBacktestingNotificationQueue, MaxMinAngles, StatisticsChartsNotifier,
    StatisticsNotifier, StrategySignals,
};
use crate::step::utils::helpers::Helpers;
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::level_utils::{
    LevelUtils, RemoveInvalidWorkingLevelsUtils, UpdateTendencyAndCreateWorkingLevelUtils,
};
use crate::step::utils::order_utils::{
    OrderUtils, UpdateOrdersBacktestingStores, UpdateOrdersBacktestingUtils,
};
use crate::step::utils::stores::{StepBacktestingMainStore, StepBacktestingStores, StepDiffs};
use crate::step::utils::StepBacktestingUtils;
use anyhow::Result;
use backtesting::trading_engine::TradingEngine;
use base::corridor::BasicCorridorUtils;
use base::entities::candle::CandleId;
use base::entities::{BasicTickProperties, Item};
use base::helpers::{Holiday, NumberOfDaysToExclude};
use base::params::StrategyParams;
use chrono::NaiveDateTime;

pub fn run_iteration<T, Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>(
    new_tick_props: BasicTickProperties,
    new_candle_props: Option<StepBacktestingCandleProperties>,
    signals: StrategySignals,
    stores: &mut StepBacktestingStores<T>,
    utils: &StepBacktestingUtils<Hel, LevUt, LevCon, OrUt, BCor, Cor, Ang, D, E, X>,
    params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
) -> Result<()>
where
    T: StepBacktestingMainStore,

    Hel: Helpers,
    LevUt: LevelUtils,
    LevCon: LevelConditions,
    OrUt: OrderUtils,
    BCor: BasicCorridorUtils,
    Cor: Corridors,
    Ang: AngleUtils,
    D: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    E: TradingEngine,
    X: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
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

    let crossed_level = LevUt::get_crossed_level(current_tick.props.bid, &created_working_levels);

    if let Some(crossed_level) = crossed_level {
        if stores
            .main
            .get_working_level_chain_of_orders(&crossed_level.id)?
            .is_empty()
        {
            let chain_of_orders = OrUt::get_new_chain_of_orders(
                crossed_level,
                params,
                stores
                    .main
                    .get_current_candle()?
                    .unwrap()
                    .props
                    .step_common
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

    LevUt::remove_active_working_levels_with_closed_orders(&mut stores.main)?;

    if let Some(current_candle) = &current_candle {
        OrUt::update_orders_backtesting(
            &current_tick.props,
            &current_candle.props,
            params,
            UpdateOrdersBacktestingStores {
                main: &mut stores.main,
                config: &mut stores.config,
                statistics: &mut stores.statistics,
            },
            UpdateOrdersBacktestingUtils::new(
                &utils.trading_engine,
                &utils.add_entity_to_chart_traces,
                &LevCon::level_exceeds_amount_of_candles_in_corridor,
                &LevCon::price_is_beyond_stop_loss,
            ),
            signals.no_trading_mode,
        )?;
    }

    LevUt::update_max_crossing_value_of_active_levels(&mut stores.main, current_tick.props.bid)?;

    if let Some(current_candle) = &current_candle {
        LevUt::remove_invalid_working_levels(
            &current_tick.props,
            current_candle.props.step_common.base.volatility,
            RemoveInvalidWorkingLevelsUtils {
                working_level_store: &mut stores.main,
                level_has_no_active_orders: &LevCon::level_has_no_active_orders,
                level_expired_by_distance: &LevCon::level_expired_by_distance,
                level_expired_by_time: &LevCon::level_expired_by_time,
                active_level_exceeds_activation_crossing_distance_when_returned_to_level: &LevCon::active_level_exceeds_activation_crossing_distance_when_returned_to_level,
                exclude_weekend_and_holidays: &utils.exclude_weekend_and_holidays,
            },
            params,
            StatisticsNotifier::<FakeBacktestingNotificationQueue>::Backtesting(
                &mut stores.statistics,
            ),
        )?;

        LevUt::move_take_profits(
            &mut stores.main,
            params.get_ratio_param_value(
                StepRatioParam::DistanceFromLevelForSignalingOfMovingTakeProfits,
                current_candle.props.step_common.base.volatility,
            ),
            params.get_ratio_param_value(
                StepRatioParam::DistanceToMoveTakeProfits,
                current_candle.props.step_common.base.volatility,
            ),
            current_tick.props.bid,
        )?;
    }

    if new_candle_appeared {
        let current_candle = current_candle.unwrap();

        Cor::update_corridors_near_working_levels(
            &mut stores.main,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
                    &BCor::candle_can_be_corridor_leader,
                    &BCor::candle_is_in_corridor,
                    &BCor::crop_corridor_to_closest_leader,
                ),
                &LevCon::level_has_no_active_orders,
            ),
            params,
        )?;

        stores.config.base.diffs.previous = stores.config.base.diffs.current;
        stores.config.base.diffs.current =
            stores.main.get_previous_candle()?.map(|previous_candle| {
                Ang::get_diff_between_current_and_previous_candles(
                    &current_candle.props,
                    &previous_candle.props,
                )
            });

        let new_angle = match stores.config.base.diffs {
            StepDiffs {
                previous: Some(previous_diff),
                current: Some(current_diff),
            } => {
                match stores.main.get_previous_candle()? {
                    Some(previous_candle) => {
                        Ang::get_new_angle(
                            &previous_candle,
                            ExistingDiffs {
                                previous: previous_diff,
                                current: current_diff,
                            },
                            MaxMinAngles {
                                max_angle: &stores.main.get_max_angle()?,
                                min_angle: &stores.main.get_min_angle()?,
                            },
                            params.get_ratio_param_value(
                                StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles,
                                current_candle.props.step_common.base.volatility
                            ),
                            params.get_ratio_param_value(
                                StepRatioParam::MinDistanceBetweenCurrentMaxAndMinAnglesForNewInnerAngleToAppear,
                                current_candle.props.step_common.base.volatility
                            ),
                        )
                    },
                    None => None
                }
            },
            _ => None,
        };

        if let Some(new_angle) = new_angle {
            Ang::update_angles(
                new_angle,
                &stores.main.get_candles_of_general_corridor()?,
                &mut stores.main,
            )?;
        }

        let max_angle = stores.main.get_max_angle()?;
        let min_angle = stores.main.get_min_angle()?;

        let crossed_angle = Ang::get_crossed_angle(
            MaxMinAngles {
                max_angle: &max_angle,
                min_angle: &min_angle,
            },
            &current_candle.props,
        );

        if let Some(crossed_angle) = crossed_angle {
            let statistics_charts_notifier: StatisticsChartsNotifier<
                FakeBacktestingNotificationQueue,
                _,
            > = StatisticsChartsNotifier::Backtesting {
                statistics: &mut stores.statistics,
                add_entity_to_chart_traces: &utils.add_entity_to_chart_traces,
                chart_traces: &mut stores.config.traces,
                current_candle_chart_index: current_candle.props.chart_index,
                crossed_angle_candle_chart_index: crossed_angle.props.candle.props.chart_index,
            };

            let create_new_working_level =
                LevUt::update_tendency_and_get_instruction_to_create_new_working_level(
                    &mut stores.config.base,
                    &mut stores.main,
                    UpdateTendencyAndCreateWorkingLevelUtils::new(
                        &LevCon::is_second_level_after_bargaining_tendency_change,
                        &LevCon::level_comes_out_of_bargaining_corridor,
                        &LevCon::appropriate_working_level,
                        &LevCon::working_level_exists,
                        &LevCon::working_level_is_close_to_another_one,
                    ),
                    statistics_charts_notifier,
                    crossed_angle,
                    &current_candle,
                    params,
                )?;
        }
    }

    Ok(())
}
