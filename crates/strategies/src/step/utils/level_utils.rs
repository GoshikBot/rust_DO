use crate::step::utils::entities::order::StepOrderProperties;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::order::{OrderStatus, OrderType};
use base::entities::tick::TickPrice;
use base::entities::Item;

use super::entities::working_levels::{BasicWLProperties, WLId};

/// Checks whether one of the working levels has got crossed and returns such a level.
pub fn get_crossed_level<W>(
    current_tick_price: TickPrice,
    created_working_levels: &[Item<WLId, W>],
) -> Option<&Item<WLId, W>>
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

fn working_level_has_closed_orders_in_chain(chain_of_orders: &[StepOrderProperties]) -> bool {
    chain_of_orders
        .iter()
        .any(|order| order.base.status == OrderStatus::Closed)
}

/// Moves active working levels to removed if they have closed orders in their chains.
pub fn remove_active_working_levels_with_closed_orders<O>(
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

        if working_level_has_closed_orders_in_chain(&level_chain_of_orders) {
            working_level_store.move_working_level_to_removed(&level.id)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use base::entities::order::{BasicOrderProperties, OrderStatus};
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

        let crossed_level = get_crossed_level(current_tick_price, &created_working_levels);

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

        let crossed_level = get_crossed_level(current_tick_price, &created_working_levels);

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

        let crossed_level = get_crossed_level(current_tick_price, &created_working_levels);

        assert!(crossed_level.is_none());
    }

    #[test]
    #[allow(non_snake_case)]
    fn remove_active_working_levels_with_closed_orders__two_active_working_levels_with_closed_orders_exist__should_move_these_two_levels_to_removed(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();
        let mut working_level_ids = Vec::new();

        for _ in 0..4 {
            working_level_ids.push(store.create_working_level(Default::default()).unwrap());
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
            })
            .collect();

        let first_chain_of_orders_without_closed_orders: Vec<_> = (0..5)
            .into_iter()
            .map(|_| store.create_order(Default::default()).unwrap())
            .collect();

        let second_chain_of_orders_without_closed_orders: Vec<_> = (0..5)
            .into_iter()
            .map(|_| store.create_order(Default::default()).unwrap())
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

        remove_active_working_levels_with_closed_orders(&mut store).unwrap();

        let removed_working_levels = store.get_removed_working_levels().unwrap();

        assert_eq!(removed_working_levels.len(), 2);
        assert!(removed_working_levels
            .iter()
            .any(|level| &level.id == working_level_ids.get(0).unwrap()));
        assert!(removed_working_levels
            .iter()
            .any(|level| &level.id == working_level_ids.get(2).unwrap()));
    }
}
