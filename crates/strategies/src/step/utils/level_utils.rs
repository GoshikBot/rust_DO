use crate::step::utils::entities::order::StepOrderProperties;
use crate::step::utils::entities::working_levels::WLMaxCrossingValue;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::order::{OrderStatus, OrderType};
use base::entities::tick::TickPrice;
use base::entities::Item;
use base::entities::TARGET_LOGGER_ENV;
use base::helpers::price_to_points;
use rust_decimal_macros::dec;

use super::entities::working_levels::{BasicWLProperties, WLId};

pub trait LevelUtils {
    /// Checks whether one of the working levels has got crossed and returns such a level.
    fn get_crossed_level<'a, W>(
        &self,
        current_tick_price: TickPrice,
        created_working_levels: &'a [Item<WLId, W>],
    ) -> Option<&'a Item<WLId, W>>
    where
        W: Into<BasicWLProperties> + Clone;

    /// Moves active working levels to removed if they have closed orders in their chains.
    fn remove_active_working_levels_with_closed_orders<O>(
        &self,
        working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
    ) -> Result<()>
    where
        O: Into<StepOrderProperties>;

    /// Updates the activation max crossing distance for active levels.
    /// It's required to delete invalid active levels that crossed particular distance
    /// and returned to level without getting to the first order.
    fn update_max_crossing_value_of_active_levels<T>(
        &self,
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
        current_tick_price: TickPrice,
    ) -> Result<()>
    where
        T: Into<BasicWLProperties>;
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
    fn get_crossed_level<'a, W>(
        &self,
        current_tick_price: TickPrice,
        created_working_levels: &'a [Item<WLId, W>],
    ) -> Option<&'a Item<WLId, W>>
    where
        W: Into<BasicWLProperties> + Clone,
    {
        for level in created_working_levels {
            let level_properties: BasicWLProperties = level.props.clone().into();

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
        &self,
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
                working_level_store.move_working_level_to_removed(&level.id)?;
            }
        }

        Ok(())
    }

    fn update_max_crossing_value_of_active_levels<T>(
        &self,
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
                target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                "current crossing value of level ({:?}) is {}",
                level, current_crossing_value
            );

            if current_crossing_value > dec!(0) {
                match working_level_store.get_max_crossing_value_of_working_level(&level.id)? {
                    None => {
                        working_level_store.update_max_crossing_value_of_working_level(
                            &level.id,
                            current_crossing_value,
                        )?;

                        log::debug!(
                            target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                            "max crossing value of level ({:?}) is set to {}",
                            level, current_crossing_value
                        );
                    }
                    Some(last_crossing_value) => {
                        log::debug!(
                            target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                            "last max crossing value of level ({:?}) is {}",
                            level, last_crossing_value
                        );

                        if current_crossing_value > last_crossing_value {
                            working_level_store.update_max_crossing_value_of_working_level(
                                &level.id,
                                current_crossing_value,
                            )?;

                            log::debug!(
                                target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                                "max crossing value of level ({:?}) is updated to {}",
                                level, current_crossing_value
                            );
                        } else {
                            log::debug!(
                                target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                                "max crossing value of level ({:?}) is not updated",
                                level
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
    use crate::step::utils::entities::working_levels::{BacktestingWLProperties, CorridorType};
    use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use base::entities::candle::CandleId;
    use base::entities::order::{BasicOrderProperties, OrderId, OrderStatus};
    use base::stores::order_store::BasicOrderStore;
    use chrono::Utc;
    use rust_decimal_macros::dec;

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

        let level_utils = LevelUtilsImpl::new();

        let crossed_level =
            level_utils.get_crossed_level(current_tick_price, &created_working_levels);

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

        let level_utils = LevelUtilsImpl::new();

        let crossed_level =
            level_utils.get_crossed_level(current_tick_price, &created_working_levels);

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

        let level_utils = LevelUtilsImpl::new();

        let crossed_level =
            level_utils.get_crossed_level(current_tick_price, &created_working_levels);

        assert!(crossed_level.is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn remove_active_working_levels_with_closed_orders__two_active_working_levels_with_closed_orders_exist__should_move_these_two_levels_to_removed(
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

        let level_utils = LevelUtilsImpl::new();

        level_utils
            .remove_active_working_levels_with_closed_orders(&mut store)
            .unwrap();

        let removed_working_levels = store.get_removed_working_levels().unwrap();

        assert_eq!(removed_working_levels.len(), 2);
        assert!(removed_working_levels
            .iter()
            .any(|level| &level.id == working_level_ids.get(0).unwrap()));
        assert!(removed_working_levels
            .iter()
            .any(|level| &level.id == working_level_ids.get(2).unwrap()));
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

        let level_utils = LevelUtilsImpl::new();

        let current_tick_price = dec!(1.37000);

        level_utils
            .update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
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

        let level_utils = LevelUtilsImpl::new();

        let current_tick_price = dec!(1.39000);

        level_utils
            .update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
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

        let level_utils = LevelUtilsImpl::new();

        let current_tick_price = dec!(1.39000);

        level_utils
            .update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
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

        let level_utils = LevelUtilsImpl::new();

        let current_tick_price = dec!(1.37000);

        level_utils
            .update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
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

        let level_utils = LevelUtilsImpl::new();

        let current_tick_price = dec!(1.37000);

        level_utils
            .update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
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

        let level_utils = LevelUtilsImpl::new();

        let current_tick_price = dec!(1.37000);

        level_utils
            .update_max_crossing_value_of_active_levels(&mut store, current_tick_price)
            .unwrap();

        assert_eq!(
            store
                .get_max_crossing_value_of_working_level(&level.id)
                .unwrap()
                .unwrap(),
            previous_max_crossing_value
        );
    }
}
