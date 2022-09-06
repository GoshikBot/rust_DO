use crate::step::utils::entities::order::StepOrderProperties;
use crate::step::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::entities::working_levels::{
    LevelTime, WLMaxCrossingValue, WLPrice, WLStatus,
};
use crate::step::utils::entities::StatisticsNotifier;
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::{Context, Result};
use base::entities::candle::CandleVolatility;
use base::entities::order::{BasicOrderProperties, OrderStatus, OrderType};
use base::entities::tick::{TickPrice, TickTime};
use base::entities::{BasicTickProperties, Item};
use base::helpers::{price_to_points, Holiday, NumberOfDaysToExclude};
use base::notifier::NotificationQueue;
use base::params::{ParamValue, StrategyParams};
use chrono::NaiveDateTime;
use rust_decimal_macros::dec;
use std::marker::PhantomData;

use super::entities::working_levels::{BasicWLProperties, WLId};

pub trait LevelUtils {
    /// Checks whether one of the working levels has got crossed and returns such a level.
    fn get_crossed_level<W>(
        current_tick_price: TickPrice,
        created_working_levels: &[Item<WLId, W>],
    ) -> Option<&Item<WLId, W>>
    where
        W: AsRef<BasicWLProperties>;

    /// Moves active working levels to removed if they have closed orders in their chains.
    fn remove_active_working_levels_with_closed_orders<O>(
        working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
    ) -> Result<()>
    where
        O: Into<StepOrderProperties>;

    /// Updates the activation max crossing distance for active levels.
    /// It's required to delete invalid active levels that crossed particular distance
    /// and returned to level without getting to the first order.
    fn update_max_crossing_value_of_active_levels<T>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
        current_tick_price: TickPrice,
    ) -> Result<()>
    where
        T: Into<BasicWLProperties>;

    fn remove_invalid_working_levels<W, A, D, M, C, E, T, N, O>(
        current_tick: &BasicTickProperties,
        current_volatility: CandleVolatility,
        utils: RemoveInvalidWorkingLevelsUtils<W, A, D, M, C, E, T, O>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        entity: StatisticsNotifier<N>,
    ) -> Result<()>
    where
        T: AsRef<BasicWLProperties>,
        O: AsRef<BasicOrderProperties>,
        W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
        A: Fn(&[O]) -> bool,
        D: Fn(WLPrice, TickPrice, ParamValue) -> bool,
        M: Fn(LevelTime, TickTime, ParamValue, &E) -> bool,
        C: Fn(&T, Option<WLMaxCrossingValue>, ParamValue, TickPrice) -> bool,
        E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
        N: NotificationQueue;

    /// Moves take profits of the existing chains of orders when the current tick price
    /// deviates from the active working level on the defined amount of points.
    fn move_take_profits<W>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        distance_from_level_for_signaling_of_moving_take_profits: ParamValue,
        distance_to_move_take_profits: ParamValue,
        current_tick_price: TickPrice,
    ) -> Result<()>
    where
        W: Into<BasicWLProperties>;
}

pub struct RemoveInvalidWorkingLevelsUtils<'a, W, A, D, M, C, E, T, O>
where
    T: AsRef<BasicWLProperties>,
    O: AsRef<BasicOrderProperties>,
    W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
    A: Fn(&[O]) -> bool,
    D: Fn(WLPrice, TickPrice, ParamValue) -> bool,
    M: Fn(LevelTime, TickTime, ParamValue, &E) -> bool,
    C: Fn(&T, Option<WLMaxCrossingValue>, ParamValue, TickPrice) -> bool,
    E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    pub working_level_store: &'a mut W,
    pub level_has_no_active_orders: &'a A,
    pub level_expired_by_distance: &'a D,
    pub level_expired_by_time: &'a M,
    pub active_level_exceeds_activation_crossing_distance_when_returned_to_level: &'a C,
    pub exclude_weekend_and_holidays: &'a E,
}

#[derive(Default)]
pub struct LevelUtilsImpl;

impl LevelUtilsImpl {
    pub fn new() -> Self {
        Self::default()
    }

    fn working_level_has_closed_orders_in_chain(chain_of_orders: &[StepOrderProperties]) -> bool {
        chain_of_orders
            .iter()
            .any(|order| order.base.status == OrderStatus::Closed)
    }
}

impl LevelUtils for LevelUtilsImpl {
    fn get_crossed_level<W>(
        current_tick_price: TickPrice,
        created_working_levels: &[Item<WLId, W>],
    ) -> Option<&Item<WLId, W>>
    where
        W: AsRef<BasicWLProperties>,
    {
        for level in created_working_levels {
            let level_properties = level.props.as_ref();

            match level_properties.r#type {
                OrderType::Buy => {
                    if current_tick_price < level_properties.price {
                        return Some(level);
                    }
                }
                OrderType::Sell => {
                    if current_tick_price > level_properties.price {
                        return Some(level);
                    }
                }
            }
        }

        None
    }

    fn remove_active_working_levels_with_closed_orders<O>(
        working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
    ) -> Result<()>
    where
        O: Into<StepOrderProperties>,
    {
        for level in working_level_store.get_active_working_levels()? {
            let level_chain_of_orders: Vec<_> = working_level_store
                .get_working_level_chain_of_orders(&level.id)?
                .into_iter()
                .map(|order| order.props.into())
                .collect();

            if Self::working_level_has_closed_orders_in_chain(&level_chain_of_orders) {
                working_level_store.remove_working_level(&level.id)?;
            }
        }

        Ok(())
    }

    fn update_max_crossing_value_of_active_levels<T>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
        current_tick_price: TickPrice,
    ) -> Result<()>
    where
        T: Into<BasicWLProperties>,
    {
        for level in working_level_store
            .get_active_working_levels()?
            .into_iter()
            .map(|level| Item {
                id: level.id,
                props: level.props.into(),
            })
        {
            let current_crossing_value = match level.props.r#type {
                OrderType::Buy => price_to_points(level.props.price - current_tick_price),
                OrderType::Sell => price_to_points(current_tick_price - level.props.price),
            };

            log::debug!(
                "current crossing value of level ({:?}) is {}",
                level,
                current_crossing_value
            );

            if current_crossing_value > dec!(0) {
                match working_level_store.get_max_crossing_value_of_working_level(&level.id)? {
                    None => {
                        working_level_store.update_max_crossing_value_of_working_level(
                            &level.id,
                            current_crossing_value,
                        )?;

                        log::debug!(
                            "max crossing value of level ({:?}) is set to {}",
                            level,
                            current_crossing_value
                        );
                    }
                    Some(last_crossing_value) => {
                        log::debug!(
                            "last max crossing value of level ({:?}) is {}",
                            level,
                            last_crossing_value
                        );

                        if current_crossing_value > last_crossing_value {
                            working_level_store.update_max_crossing_value_of_working_level(
                                &level.id,
                                current_crossing_value,
                            )?;

                            log::debug!(
                                "max crossing value of level ({:?}) is updated to {}",
                                level,
                                current_crossing_value
                            );
                        } else {
                            log::debug!("max crossing value of level ({:?}) is not updated", level);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn remove_invalid_working_levels<W, A, D, M, C, E, T, N, O>(
        current_tick: &BasicTickProperties,
        current_volatility: CandleVolatility,
        utils: RemoveInvalidWorkingLevelsUtils<W, A, D, M, C, E, T, O>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        mut entity: StatisticsNotifier<N>,
    ) -> Result<()>
    where
        T: AsRef<BasicWLProperties>,
        O: AsRef<BasicOrderProperties>,
        W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
        A: Fn(&[O]) -> bool,
        D: Fn(WLPrice, TickPrice, ParamValue) -> bool,
        M: Fn(LevelTime, TickTime, ParamValue, &E) -> bool,
        C: Fn(&T, Option<WLMaxCrossingValue>, ParamValue, TickPrice) -> bool,
        E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
        N: NotificationQueue,
    {
        for level in utils
            .working_level_store
            .get_created_working_levels()?
            .into_iter()
            .chain(
                utils
                    .working_level_store
                    .get_active_working_levels()?
                    .into_iter(),
            )
        {
            let converted_level = Item {
                id: &level.id,
                props: level.props.as_ref(),
            };

            let level_status = utils
                .working_level_store
                .get_working_level_status(&level.id)?
                .unwrap();

            let mut remove_level = false;

            let distance_from_level_for_its_deletion = params.get_ratio_param_value(
                StepRatioParam::DistanceFromLevelForItsDeletion,
                current_volatility,
            );

            if level_status == WLStatus::Created
                || (level_status == WLStatus::Active
                    && (utils.level_has_no_active_orders)(
                        &utils
                            .working_level_store
                            .get_working_level_chain_of_orders(&level.id)?
                            .into_iter()
                            .map(|order| order.props)
                            .collect::<Vec<_>>(),
                    ))
            {
                if (utils.level_expired_by_distance)(
                    converted_level.props.price,
                    current_tick.bid,
                    distance_from_level_for_its_deletion,
                ) {
                    log::debug!("level ({:?}) is expired by distance", converted_level);

                    match &mut entity {
                        StatisticsNotifier::Backtesting(statistics) => {
                            statistics.deleted_by_expiration_by_distance += 1;
                        }
                        StatisticsNotifier::Realtime(queue) => {
                            queue.send_message(format!(
                                "level ({:?}) is expired by distance",
                                converted_level
                            ))?;
                        }
                    }

                    remove_level = true;
                } else {
                    log::debug!("level ({:?}) is NOT expired by distance", converted_level);

                    let level_expiration =
                        params.get_point_param_value(StepPointParam::LevelExpirationDays);

                    if (utils.level_expired_by_time)(
                        converted_level.props.time,
                        current_tick.time,
                        level_expiration,
                        utils.exclude_weekend_and_holidays,
                    ) {
                        log::debug!("level ({:?}) is expired by time", converted_level);

                        match &mut entity {
                            StatisticsNotifier::Backtesting(statistics) => {
                                statistics.deleted_by_expiration_by_time += 1;
                            }
                            StatisticsNotifier::Realtime(queue) => {
                                queue.send_message(format!(
                                    "level ({:?}) is expired by time",
                                    converted_level
                                ))?;
                            }
                        }

                        remove_level = true;
                    } else {
                        log::debug!("level ({:?}) is NOT expired by time", converted_level);

                        if level_status == WLStatus::Active {
                            let max_crossing_value = utils
                                .working_level_store
                                .get_max_crossing_value_of_working_level(&level.id)?;

                            let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion = params.get_ratio_param_value(
                                StepRatioParam::MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion,
                                current_volatility
                            );

                            if (utils.active_level_exceeds_activation_crossing_distance_when_returned_to_level)(
                                &level.props,
                                max_crossing_value,
                                min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
                                current_tick.bid
                            ) {
                                log::debug!(
                                    "level ({:?}) exceeds activation crossing distance when returned to level: {:?} >= {}",
                                    converted_level, max_crossing_value,
                                    min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion
                                );

                                match &mut entity {
                                    StatisticsNotifier::Backtesting(statistics) => {
                                        statistics.deleted_by_exceeding_activation_crossing_distance += 1;
                                    }
                                    StatisticsNotifier::Realtime(queue) => {
                                        queue.send_message(format!(
                                            "level ({:?}) exceeds activation crossing distance when returned to level: {:?} >= {}",
                                            converted_level, max_crossing_value,
                                            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion
                                        ))?;
                                    }
                                }

                                remove_level = true;
                            } else {
                                log::debug!(
                                    "level ({:?}) DOES NOT exceed activation crossing distance when returned to level: {:?} < {}",
                                    converted_level, max_crossing_value,
                                    min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion
                                );
                            }
                        }
                    }
                }
            }

            if remove_level {
                utils.working_level_store.remove_working_level(&level.id)?;

                if let StatisticsNotifier::Backtesting(statistics) = &mut entity {
                    statistics.number_of_working_levels -= 1;
                }
            }
        }

        Ok(())
    }

    fn move_take_profits<W>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        distance_from_level_for_signaling_of_moving_take_profits: ParamValue,
        distance_to_move_take_profits: ParamValue,
        current_tick_price: TickPrice,
    ) -> Result<()>
    where
        W: Into<BasicWLProperties>,
    {
        for level in working_level_store
            .get_active_working_levels()?
            .into_iter()
            .map(|level| Item {
                id: level.id,
                props: level.props.into(),
            })
        {
            let deviation_distance = match level.props.r#type {
                OrderType::Buy => price_to_points(level.props.price - current_tick_price),
                OrderType::Sell => price_to_points(current_tick_price - level.props.price),
            };

            if deviation_distance >= distance_from_level_for_signaling_of_moving_take_profits {
                log::debug!(
                    "move take profits of level ({:?}), because the deviation distance ({}) \
                    >= distance from level for signaling of moving take profits ({})",
                    level,
                    deviation_distance,
                    distance_from_level_for_signaling_of_moving_take_profits
                );

                working_level_store
                    .move_take_profits_of_level(&level.id, distance_to_move_take_profits)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::step::utils::entities::working_levels::{
        BacktestingWLProperties, CorridorType, LevelTime, WLPrice,
    };
    use crate::step::utils::entities::FakeBacktestingNotificationQueue;
    use crate::step::utils::level_conditions::{LevelConditionsImpl, MinAmountOfCandles};
    use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use crate::step::utils::stores::StepBacktestingStatistics;
    use base::entities::order::{BasicOrderPrices, BasicOrderProperties, OrderPrice, OrderStatus};
    use base::entities::tick::TickTime;
    use base::helpers::points_to_price;
    use base::notifier::Message;
    use base::params::ParamValue;
    use base::stores::order_store::BasicOrderStore;
    use chrono::{Datelike, NaiveDate, Utc};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::cell::RefCell;

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn get_crossed_level__current_tick_price_is_less_than_buy_level_price__should_return_buy_level()
    {
        let created_working_levels = vec![
            Item {
                id: String::from("2"),
                props: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(10),
                    time: Utc::now().naive_utc(),
                },
            },
            Item {
                id: String::from("1"),
                props: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(10),
                    time: Utc::now().naive_utc(),
                },
            },
        ];

        let current_tick_price = dec!(9);

        let crossed_level =
            LevelUtilsImpl::get_crossed_level(current_tick_price, &created_working_levels);

        assert_eq!(crossed_level.unwrap().id, "1");
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_crossed_level__current_tick_price_is_greater_than_sell_level_price__should_return_sell_level(
    ) {
        let created_working_levels = vec![
            Item {
                id: String::from("1"),
                props: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(10),
                    time: Utc::now().naive_utc(),
                },
            },
            Item {
                id: String::from("2"),
                props: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(10),
                    time: Utc::now().naive_utc(),
                },
            },
        ];

        let current_tick_price = dec!(11);

        let crossed_level =
            LevelUtilsImpl::get_crossed_level(current_tick_price, &created_working_levels);

        assert_eq!(crossed_level.unwrap().id, "2");
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_crossed_level__current_tick_price_is_greater_than_buy_level_price_and_less_than_sell_level_price__should_return_none(
    ) {
        let created_working_levels = vec![
            Item {
                id: String::from("1"),
                props: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(10),
                    time: Utc::now().naive_utc(),
                },
            },
            Item {
                id: String::from("2"),
                props: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(12),
                    time: Utc::now().naive_utc(),
                },
            },
        ];

        let current_tick_price = dec!(11);

        let crossed_level =
            LevelUtilsImpl::get_crossed_level(current_tick_price, &created_working_levels);

        assert!(crossed_level.is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn remove_active_working_levels_with_closed_orders__two_active_working_levels_with_closed_orders_exist__should_remove_these_two_levels(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();
        let mut working_level_ids = Vec::new();

        for _ in 0..4 {
            working_level_ids.push(store.create_working_level(Default::default()).unwrap().id);
        }

        let first_chain_of_orders_with_closed_orders: Vec<_> = (0..5)
            .into_iter()
            .map(|i| {
                let status = if i > 2 {
                    OrderStatus::Closed
                } else {
                    OrderStatus::Pending
                };

                store
                    .create_order(StepOrderProperties {
                        base: BasicOrderProperties {
                            status,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .unwrap()
                    .id
            })
            .collect();

        let second_chain_of_orders_with_closed_orders: Vec<_> = (0..5)
            .into_iter()
            .map(|i| {
                let status = if i > 3 {
                    OrderStatus::Closed
                } else {
                    OrderStatus::Opened
                };

                store
                    .create_order(StepOrderProperties {
                        base: BasicOrderProperties {
                            status,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .unwrap()
                    .id
            })
            .collect();

        let first_chain_of_orders_without_closed_orders: Vec<_> = (0..5)
            .into_iter()
            .map(|_| store.create_order(Default::default()).unwrap().id)
            .collect();

        let second_chain_of_orders_without_closed_orders: Vec<_> = (0..5)
            .into_iter()
            .map(|_| store.create_order(Default::default()).unwrap().id)
            .collect();

        for order_id in first_chain_of_orders_with_closed_orders {
            store
                .add_order_to_working_level_chain_of_orders(
                    working_level_ids.get(0).unwrap(),
                    order_id,
                )
                .unwrap();
        }

        for order_id in second_chain_of_orders_with_closed_orders {
            store
                .add_order_to_working_level_chain_of_orders(
                    working_level_ids.get(2).unwrap(),
                    order_id,
                )
                .unwrap();
        }

        for order_id in first_chain_of_orders_without_closed_orders {
            store
                .add_order_to_working_level_chain_of_orders(
                    working_level_ids.get(1).unwrap(),
                    order_id,
                )
                .unwrap();
        }

        for order_id in second_chain_of_orders_without_closed_orders {
            store
                .add_order_to_working_level_chain_of_orders(
                    working_level_ids.get(3).unwrap(),
                    order_id,
                )
                .unwrap();
        }

        for level_id in working_level_ids.iter() {
            store.move_working_level_to_active(level_id).unwrap();
        }

        LevelUtilsImpl::remove_active_working_levels_with_closed_orders(&mut store).unwrap();

        assert!(!store
            .get_active_working_levels()
            .unwrap()
            .iter()
            .any(|level| { level.id == working_level_ids[0] || level.id == working_level_ids[2] }));
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_max_crossing_value_of_level__buy_level_first_crossing_value__should_set_new_crossing_value(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_price = dec!(1.38000);

        let level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: level_price,
                    ..Default::default()
                },
                chart_index: 0,
            })
            .unwrap();

        store.move_working_level_to_active(&level.id).unwrap();

        let current_tick_price = dec!(1.37000);

        LevelUtilsImpl::update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        let expected_max_crossing_value = price_to_points(level_price - current_tick_price);

        assert_eq!(
            store
                .get_max_crossing_value_of_working_level(&level.id)
                .unwrap()
                .unwrap(),
            expected_max_crossing_value
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_max_crossing_value_of_level__sell_level_first_crossing_value__should_set_new_crossing_value(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_price = dec!(1.38000);

        let level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: level_price,
                    ..Default::default()
                },
                chart_index: 0,
            })
            .unwrap();

        store.move_working_level_to_active(&level.id).unwrap();

        let current_tick_price = dec!(1.39000);

        LevelUtilsImpl::update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        let expected_max_crossing_value = price_to_points(current_tick_price - level_price);

        assert_eq!(
            store
                .get_max_crossing_value_of_working_level(&level.id)
                .unwrap()
                .unwrap(),
            expected_max_crossing_value
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_max_crossing_value_of_level__buy_level_crossing_value_is_negative__should_not_set_new_crossing_value(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_price = dec!(1.38000);

        let level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: level_price,
                    ..Default::default()
                },
                chart_index: 0,
            })
            .unwrap();

        store.move_working_level_to_active(&level.id).unwrap();

        let current_tick_price = dec!(1.39000);

        LevelUtilsImpl::update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        assert!(store
            .get_max_crossing_value_of_working_level(&level.id)
            .unwrap()
            .is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_max_crossing_value_of_level__sell_level_crossing_value_is_negative__should_not_set_new_crossing_value(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_price = dec!(1.38000);

        let level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: level_price,
                    ..Default::default()
                },
                chart_index: 0,
            })
            .unwrap();

        store.move_working_level_to_active(&level.id).unwrap();

        let current_tick_price = dec!(1.37000);

        LevelUtilsImpl::update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        assert!(store
            .get_max_crossing_value_of_working_level(&level.id)
            .unwrap()
            .is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_max_crossing_value_of_level__crossing_value_is_greater_than_previous__should_update_crossing_value(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_price = dec!(1.38000);

        let level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: level_price,
                    ..Default::default()
                },
                chart_index: 0,
            })
            .unwrap();

        store.move_working_level_to_active(&level.id).unwrap();
        store
            .update_max_crossing_value_of_working_level(&level.id, dec!(200))
            .unwrap();

        let current_tick_price = dec!(1.37000);

        LevelUtilsImpl::update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        assert_eq!(
            store
                .get_max_crossing_value_of_working_level(&level.id)
                .unwrap()
                .unwrap(),
            price_to_points(level_price - current_tick_price)
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_max_crossing_value_of_level__crossing_value_is_less_than_previous__should_not_update_crossing_value(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_price = dec!(1.38000);

        let level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: level_price,
                    ..Default::default()
                },
                chart_index: 0,
            })
            .unwrap();

        store.move_working_level_to_active(&level.id).unwrap();
        let previous_max_crossing_value = dec!(2000);

        store
            .update_max_crossing_value_of_working_level(&level.id, previous_max_crossing_value)
            .unwrap();

        let current_tick_price = dec!(1.37000);

        LevelUtilsImpl::update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        assert_eq!(
            store
                .get_max_crossing_value_of_working_level(&level.id)
                .unwrap()
                .unwrap(),
            previous_max_crossing_value
        );
    }

    #[derive(Default)]
    struct TestLevelConditionsImpl;

    impl LevelConditions for TestLevelConditionsImpl {
        fn level_exceeds_amount_of_candles_in_corridor(
            _level_id: &str,
            _working_level_store: &impl StepWorkingLevelStore,
            _corridor_type: CorridorType,
            _min_amount_of_candles: MinAmountOfCandles,
        ) -> Result<bool> {
            todo!()
        }

        fn price_is_beyond_stop_loss(
            _current_tick_price: TickPrice,
            _stop_loss_price: OrderPrice,
            _working_level_type: OrderType,
        ) -> bool {
            todo!()
        }

        fn level_expired_by_distance(
            level_price: WLPrice,
            _current_tick_price: TickPrice,
            _distance_from_level_for_its_deletion: ParamValue,
        ) -> bool {
            level_price == dec!(1) || level_price == dec!(5)
        }

        fn level_expired_by_time(
            level_time: LevelTime,
            _current_tick_time: TickTime,
            _level_expiration: ParamValue,
            _exclude_weekend_and_holidays: &impl Fn(
                NaiveDateTime,
                NaiveDateTime,
                &[Holiday],
            ) -> NumberOfDaysToExclude,
        ) -> bool {
            matches!(level_time.day(), 2 | 6)
        }

        fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            level: &impl AsRef<BasicWLProperties>,
            _max_crossing_value: Option<WLMaxCrossingValue>,
            _min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
            _current_tick_price: TickPrice,
        ) -> bool {
            level.as_ref().price == dec!(7)
        }

        fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
            level_orders.is_empty()
        }
    }

    #[derive(Default)]
    struct TestStrategyParams;

    impl StrategyParams for TestStrategyParams {
        type PointParam = StepPointParam;
        type RatioParam = StepRatioParam;

        fn get_point_param_value(&self, _name: Self::PointParam) -> ParamValue {
            dec!(2)
        }

        fn get_ratio_param_value(
            &self,
            _name: Self::RatioParam,
            _volatility: CandleVolatility,
        ) -> ParamValue {
            dec!(2)
        }
    }

    fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
        level_orders.is_empty()
    }

    fn level_expired_by_distance(
        level_price: WLPrice,
        _current_tick_price: TickPrice,
        _distance_from_level_for_its_deletion: ParamValue,
    ) -> bool {
        level_price == dec!(1) || level_price == dec!(5)
    }

    fn level_expired_by_time(
        level_time: LevelTime,
        _current_tick_time: TickTime,
        _level_expiration: ParamValue,
        _exclude_weekend_and_holidays: &impl Fn(
            NaiveDateTime,
            NaiveDateTime,
            &[Holiday],
        ) -> NumberOfDaysToExclude,
    ) -> bool {
        matches!(level_time.day(), 2 | 6)
    }

    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        level: &impl AsRef<BasicWLProperties>,
        _max_crossing_value: Option<WLMaxCrossingValue>,
        _min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
        _current_tick_price: TickPrice,
    ) -> bool {
        level.as_ref().price == dec!(7)
    }

    #[test]
    #[allow(non_snake_case)]
    fn remove_invalid_working_levels__backtesting__should_remove_only_invalid_levels() {
        let mut store = InMemoryStepBacktestingStore::new();

        let level_utils = LevelUtilsImpl::new();

        let current_tick = BasicTickProperties::default();
        let current_volatility = 280;

        let exclude_weekend_and_holidays =
            |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 0;

        let params = TestStrategyParams::default();
        let mut statistics = StepBacktestingStatistics {
            number_of_working_levels: 8,
            ..Default::default()
        };

        // Notation
        // d — expired by distance
        // t — expired by time
        // o — has no active orders
        // c — exceeds activation crossing distance when returned to level

        // Working level local indexes
        // created:
        //  - 1 (d)
        //  - 2 (t)
        //  - 3 (!d && !t)
        //
        // active:
        //  - 4 (!o)
        //  - 5 (o && d)
        //  - 6 (o && t)
        //  - 7 (o && c)
        //  - 8 (o && !d && !t && !c)

        for i in 1..=8 {
            let level = store
                .create_working_level(BacktestingWLProperties {
                    base: BasicWLProperties {
                        price: Decimal::from(i),
                        time: NaiveDate::from_ymd(2022, 1, i).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .unwrap();

            if i == 4 {
                let order = store.create_order(Default::default()).unwrap();
                store
                    .add_order_to_working_level_chain_of_orders(&level.id, order.id)
                    .unwrap();
            }

            if i > 3 {
                store.move_working_level_to_active(&level.id).unwrap();
            }
        }

        LevelUtilsImpl::remove_invalid_working_levels(
            &current_tick,
            current_volatility,
            RemoveInvalidWorkingLevelsUtils {
                working_level_store: &mut store,
                level_has_no_active_orders: &level_has_no_active_orders,
                level_expired_by_distance: &level_expired_by_distance,
                level_expired_by_time: &level_expired_by_time,
                active_level_exceeds_activation_crossing_distance_when_returned_to_level:
                    &active_level_exceeds_activation_crossing_distance_when_returned_to_level,
                exclude_weekend_and_holidays: &exclude_weekend_and_holidays,
            },
            &params,
            StatisticsNotifier::<FakeBacktestingNotificationQueue>::Backtesting(&mut statistics),
        )
        .unwrap();

        assert_eq!(statistics.number_of_working_levels, 3);
        assert_eq!(store.get_created_working_levels().unwrap().len(), 1);
        assert_eq!(store.get_active_working_levels().unwrap().len(), 2);

        assert_eq!(statistics.deleted_by_expiration_by_distance, 2);
        assert_eq!(statistics.deleted_by_expiration_by_time, 2);
        assert_eq!(
            statistics.deleted_by_exceeding_activation_crossing_distance,
            1
        );
    }

    #[derive(Default)]
    struct TestNotificationQueue {
        number_of_calls: RefCell<u32>,
    }

    impl NotificationQueue for TestNotificationQueue {
        fn send_message(&self, _message: Message) -> Result<()> {
            *self.number_of_calls.borrow_mut() += 1;
            Ok(())
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn remove_invalid_working_levels__realtime__should_remove_only_invalid_levels() {
        let mut store = InMemoryStepBacktestingStore::new();

        let current_tick = BasicTickProperties::default();
        let current_volatility = 280;

        let level_conditions = TestLevelConditionsImpl::default();
        let exclude_weekend_and_holidays =
            |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 0;

        let params = TestStrategyParams::default();

        // Notation
        // d — expired by distance
        // t — expired by time
        // o — has no active orders
        // c — exceeds activation crossing distance when returned to level

        // Working level local indexes
        // created:
        //  - 1 (d)
        //  - 2 (t)
        //  - 3 (!d && !t)
        //
        // active:
        //  - 4 (!o)
        //  - 5 (o && d)
        //  - 6 (o && t)
        //  - 7 (o && c)
        //  - 8 (o && !d && !t && !c)

        for i in 1..=8 {
            let level = store
                .create_working_level(BacktestingWLProperties {
                    base: BasicWLProperties {
                        price: Decimal::from(i),
                        time: NaiveDate::from_ymd(2022, 1, i).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .unwrap();

            if i == 4 {
                let order = store.create_order(Default::default()).unwrap();
                store
                    .add_order_to_working_level_chain_of_orders(&level.id, order.id)
                    .unwrap();
            }

            if i > 3 {
                store.move_working_level_to_active(&level.id).unwrap();
            }
        }

        let notification_queue = TestNotificationQueue::default();

        LevelUtilsImpl::remove_invalid_working_levels(
            &current_tick,
            current_volatility,
            RemoveInvalidWorkingLevelsUtils {
                working_level_store: &mut store,
                level_has_no_active_orders: &level_has_no_active_orders,
                level_expired_by_distance: &level_expired_by_distance,
                level_expired_by_time: &level_expired_by_time,
                active_level_exceeds_activation_crossing_distance_when_returned_to_level:
                    &active_level_exceeds_activation_crossing_distance_when_returned_to_level,
                exclude_weekend_and_holidays: &exclude_weekend_and_holidays,
            },
            &params,
            StatisticsNotifier::Realtime(&notification_queue),
        )
        .unwrap();

        assert_eq!(store.get_created_working_levels().unwrap().len(), 1);
        assert_eq!(store.get_active_working_levels().unwrap().len(), 2);
        assert_eq!(*notification_queue.number_of_calls.borrow(), 5);
    }

    #[test]
    #[allow(non_snake_case)]
    fn move_take_profits__should_successfully_move_take_profits_of_active_levels_only() {
        let level_utils = LevelUtilsImpl::new();

        let mut store = InMemoryStepBacktestingStore::new();

        let take_profit = dec!(1.36800);
        let buy_wl_price = dec!(1.37000);
        let current_tick_price = dec!(1.36799);
        let sell_wl_price = dec!(1.36598);

        for i in 0..8 {
            let level = store
                .create_working_level(BacktestingWLProperties {
                    base: BasicWLProperties {
                        price: if i <= 2 { buy_wl_price } else { sell_wl_price },
                        r#type: if i <= 2 {
                            OrderType::Buy
                        } else {
                            OrderType::Sell
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .unwrap();

            for _ in 0..3 {
                store
                    .create_order(StepOrderProperties {
                        base: BasicOrderProperties {
                            prices: BasicOrderPrices {
                                take_profit,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        working_level_id: level.id.clone(),
                    })
                    .unwrap();
            }

            if i <= 5 {
                store.move_working_level_to_active(&level.id).unwrap();
            }
        }

        let distance_from_level_for_signaling_of_moving_take_profits = dec!(200);
        let distance_to_move_take_profits = dec!(30);

        LevelUtilsImpl::move_take_profits(
            &mut store,
            distance_from_level_for_signaling_of_moving_take_profits,
            distance_to_move_take_profits,
            current_tick_price,
        )
        .unwrap();

        for level in store.get_active_working_levels().unwrap() {
            for order in store.get_working_level_chain_of_orders(&level.id).unwrap() {
                match level.props.base.r#type {
                    OrderType::Buy => assert_eq!(
                        order.props.base.prices.take_profit,
                        take_profit - points_to_price(distance_to_move_take_profits)
                    ),
                    OrderType::Sell => assert_eq!(
                        order.props.base.prices.take_profit,
                        take_profit + points_to_price(distance_to_move_take_profits)
                    ),
                }
            }
        }

        for level in store.get_created_working_levels().unwrap() {
            for order in store.get_working_level_chain_of_orders(&level.id).unwrap() {
                assert_eq!(order.props.base.prices.take_profit, take_profit);
            }
        }
    }
}
