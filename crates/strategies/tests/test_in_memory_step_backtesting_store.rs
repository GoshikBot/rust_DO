use rust_decimal_macros::dec;
use std::collections::HashSet;

use base::entities::candle::CandleId;
use base::entities::order::{BasicOrderPrices, BasicOrderProperties, OrderStatus, OrderType};
use base::entities::tick::TickId;
use base::entities::Level;
use base::helpers::points_to_price;
use base::stores::candle_store::BasicCandleStore;
use base::stores::order_store::BasicOrderStore;
use base::stores::tick_store::BasicTickStore;
use strategies::step::utils::entities::angle::{AngleId, AngleState, BasicAngleProperties};
use strategies::step::utils::entities::order::StepOrderProperties;
use strategies::step::utils::entities::working_levels::{
    BacktestingWLProperties, BasicWLProperties, CorridorType, WLStatus,
};
use strategies::step::utils::stores::angle_store::StepAngleStore;
use strategies::step::utils::stores::candle_store::StepCandleStore;
use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use strategies::step::utils::stores::tick_store::StepTickStore;
use strategies::step::utils::stores::working_level_store::StepWorkingLevelStore;

#[test]
fn should_remove_only_unused_items() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let mut ticks = Vec::new();

    for _ in 1..=4 {
        let tick_id = store
            .create_tick(xid::new().to_string(), Default::default())
            .unwrap()
            .id;
        ticks.push(tick_id);
    }

    assert!(store
        .update_current_tick(ticks.get(0).unwrap().clone())
        .is_ok());
    assert!(store
        .update_previous_tick(ticks.get(1).unwrap().clone())
        .is_ok());

    let mut candles = Vec::new();
    for _ in 1..=12 {
        let candle_id = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap()
            .id;
        candles.push(candle_id);
    }

    assert!(store
        .update_current_candle(candles.get(0).unwrap().clone())
        .is_ok());
    assert!(store
        .update_previous_candle(candles.get(1).unwrap().clone())
        .is_ok());

    let working_level_id = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candles.get(2).unwrap().clone(),
            CorridorType::Small,
        )
        .unwrap();

    store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candles.get(8).unwrap().clone(),
            CorridorType::Small,
        )
        .unwrap();

    store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candles.get(3).unwrap().clone(),
            CorridorType::Big,
        )
        .unwrap();

    store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candles.get(9).unwrap().clone(),
            CorridorType::Big,
        )
        .unwrap();

    store
        .add_candle_to_general_corridor(candles[10].clone())
        .unwrap();

    let mut angles = Vec::new();

    for i in 1..=10 {
        let angle_id = store
            .create_angle(
                xid::new().to_string(),
                BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candles.get(i - 1).unwrap().clone(),
            )
            .unwrap()
            .id;
        angles.push(angle_id);
    }

    assert!(store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angles.get(0).unwrap().clone()
        ))
        .is_ok());
    assert!(store
        .update_tendency_change_angle(angles.get(1).unwrap().clone())
        .is_ok());
    assert!(store
        .update_min_angle(angles.get(2).unwrap().clone())
        .is_ok());
    assert!(store
        .update_max_angle(angles.get(3).unwrap().clone())
        .is_ok());
    assert!(store
        .update_virtual_min_angle(angles.get(4).unwrap().clone())
        .is_ok());
    assert!(store
        .update_virtual_max_angle(angles.get(5).unwrap().clone())
        .is_ok());
    assert!(store
        .update_min_angle_before_bargaining_corridor(angles.get(6).unwrap().clone())
        .is_ok());
    assert!(store
        .update_max_angle_before_bargaining_corridor(angles.get(7).unwrap().clone())
        .is_ok());

    store.remove_unused_items().unwrap();

    let left_ticks = ticks.drain(0..2).collect::<HashSet<_>>();

    assert!(store
        .get_all_ticks()
        .unwrap()
        .symmetric_difference(&left_ticks)
        .collect::<HashSet<&TickId>>()
        .is_empty());

    let left_candles = candles[0..=10].iter().cloned().collect::<HashSet<_>>();

    assert!(store
        .get_all_candles()
        .unwrap()
        .symmetric_difference(&left_candles)
        .collect::<HashSet<&CandleId>>()
        .is_empty());

    let left_angles = angles.drain(0..=7).collect::<HashSet<_>>();

    assert!(store
        .get_all_angles()
        .unwrap()
        .symmetric_difference(&left_angles)
        .collect::<HashSet<&AngleId>>()
        .is_empty());

    store
        .clear_working_level_corridor(&working_level_id, CorridorType::Small)
        .unwrap();
    store
        .clear_working_level_corridor(&working_level_id, CorridorType::Big)
        .unwrap();

    store.clear_general_corridor().unwrap();

    store.remove_unused_items().unwrap();

    assert!(
        !store.get_all_candles().unwrap().contains(&candles[8])
            && !store.get_all_candles().unwrap().contains(&candles[9])
            && !store.get_all_candles().unwrap().contains(&candles[10])
    );
}

#[test]
fn should_return_error_on_moving_working_level_to_active_if_it_is_not_present_in_created() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    assert!(store
        .move_working_level_to_active(&working_level_id)
        .is_ok());
    assert!(store
        .move_working_level_to_active(&working_level_id)
        .is_err());
}

#[test]
fn should_successfully_remove_working_level() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    assert!(store
        .move_working_level_to_active(&working_level_id)
        .is_ok());

    assert!(store
        .update_max_crossing_value_of_working_level(&working_level_id, dec!(10))
        .is_ok());

    let candle_id = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candle_id.clone(),
            CorridorType::Small
        )
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor(&working_level_id, candle_id, CorridorType::Big)
        .is_ok());

    assert!(store
        .move_take_profits_of_level(&working_level_id, dec!(100))
        .is_ok());

    let order_id = store
        .create_order(
            xid::new().to_string(),
            StepOrderProperties {
                working_level_id: working_level_id.clone(),
                ..Default::default()
            },
        )
        .unwrap()
        .id;

    store.remove_working_level(&working_level_id).unwrap();

    assert!(!store
        .get_created_working_levels()
        .unwrap()
        .iter()
        .any(|level| level.id == working_level_id));

    assert!(!store
        .get_active_working_levels()
        .unwrap()
        .iter()
        .any(|level| level.id == working_level_id));

    assert!(store
        .get_candles_of_working_level_corridor(&working_level_id, CorridorType::Small)
        .unwrap()
        .is_empty());

    assert!(store
        .get_candles_of_working_level_corridor(&working_level_id, CorridorType::Big)
        .unwrap()
        .is_empty());

    assert!(!store
        .take_profits_of_level_are_moved(&working_level_id)
        .unwrap());

    assert!(store
        .get_max_crossing_value_of_working_level(&working_level_id)
        .unwrap()
        .is_none());

    assert!(store
        .get_working_level_chain_of_orders(&working_level_id)
        .unwrap()
        .is_empty());
}

#[test]
fn should_successfully_get_all_working_levels() {
    let mut store = InMemoryStepBacktestingStore::default();

    let created_level = store
        .create_working_level(String::from("1"), Default::default())
        .unwrap();

    let active_level = store
        .create_working_level(String::from("2"), Default::default())
        .unwrap();
    store
        .move_working_level_to_active(&active_level.id)
        .unwrap();

    let all_levels = store.get_all_working_levels().unwrap();

    assert_eq!(all_levels.len(), 2);
    assert!(all_levels.contains(&created_level));
    assert!(all_levels.contains(&active_level));
}

#[test]
fn should_successfully_add_candle_to_working_level_corridor_and_then_clear_corridor() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    let first_candle_id = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    let second_candle_id = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            first_candle_id.clone(),
            CorridorType::Small
        )
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            second_candle_id.clone(),
            CorridorType::Big
        )
        .is_ok());

    assert!(store
        .get_candles_of_working_level_corridor(&working_level_id, CorridorType::Small)
        .unwrap()
        .iter()
        .any(|candle| candle.id == first_candle_id));

    assert!(store
        .get_candles_of_working_level_corridor(&working_level_id, CorridorType::Big)
        .unwrap()
        .iter()
        .any(|candle| candle.id == second_candle_id));

    store
        .clear_working_level_corridor(&working_level_id, CorridorType::Small)
        .unwrap();
    assert!(store
        .get_candles_of_working_level_corridor(&working_level_id, CorridorType::Small)
        .unwrap()
        .is_empty());

    store
        .clear_working_level_corridor(&working_level_id, CorridorType::Big)
        .unwrap();
    assert!(store
        .get_candles_of_working_level_corridor(&working_level_id, CorridorType::Big)
        .unwrap()
        .is_empty());
}

#[test]
fn should_correctly_update_general_corridor() {
    let mut store = InMemoryStepBacktestingStore::default();

    let candle_id = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    assert!(store.get_candles_of_general_corridor().unwrap().is_empty());

    store
        .add_candle_to_general_corridor(candle_id.clone())
        .unwrap();

    let candles = store.get_candles_of_general_corridor().unwrap();

    assert!(candles.len() == 1 && candles[0].id == candle_id);

    store.clear_general_corridor().unwrap();

    assert!(store.get_candles_of_general_corridor().unwrap().is_empty());
}

#[test]
fn should_return_error_on_adding_candle_to_working_level_corridor_if_it_is_already_present_there() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    let candle_id = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candle_id.clone(),
            CorridorType::Small
        )
        .is_ok());
    assert!(store
        .add_candle_to_working_level_corridor(&working_level_id, candle_id, CorridorType::Small)
        .is_err());
}

#[test]
fn should_successfully_create_order_with_existing_working_level() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap()
        .id;

    let order_id = store
        .create_order(
            xid::new().to_string(),
            StepOrderProperties {
                working_level_id: working_level_id.clone(),
                ..Default::default()
            },
        )
        .unwrap()
        .id;

    assert!(store
        .get_working_level_chain_of_orders(&working_level_id)
        .unwrap()
        .iter()
        .any(|order| order.id == order_id));
}

#[test]
fn should_return_error_on_creating_order_with_nonexistent_working_level() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = String::from("nonexistent");

    assert!(store
        .create_order(
            xid::new().to_string(),
            StepOrderProperties {
                working_level_id,
                ..Default::default()
            }
        )
        .is_err());
}

#[test]
fn should_successfully_identify_level_status() {
    let mut store = InMemoryStepBacktestingStore::default();

    let created_working_level = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap();

    let active_working_level = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap();
    store
        .move_working_level_to_active(&active_working_level.id)
        .unwrap();

    assert_eq!(
        store
            .get_working_level_status(&created_working_level.id)
            .unwrap()
            .unwrap(),
        WLStatus::Created
    );
    assert_eq!(
        store
            .get_working_level_status(&active_working_level.id)
            .unwrap()
            .unwrap(),
        WLStatus::Active
    );
    assert!(store
        .get_working_level_status("nonexistent level")
        .unwrap()
        .is_none());
}

#[test]
fn should_successfully_move_take_profits_of_level() {
    let mut store = InMemoryStepBacktestingStore::default();

    let buy_working_level_id = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap()
        .id;

    let sell_working_level_id = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap()
        .id;

    let take_profit = dec!(1.38000);

    for _ in 0..5 {
        store
            .create_order(
                xid::new().to_string(),
                StepOrderProperties {
                    base: BasicOrderProperties {
                        prices: BasicOrderPrices {
                            take_profit,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: buy_working_level_id.clone(),
                },
            )
            .unwrap();

        store
            .create_order(
                xid::new().to_string(),
                StepOrderProperties {
                    base: BasicOrderProperties {
                        prices: BasicOrderPrices {
                            take_profit,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: sell_working_level_id.clone(),
                },
            )
            .unwrap();
    }

    let distance_to_move_take_profits = dec!(100);

    store
        .move_take_profits_of_level(&buy_working_level_id, distance_to_move_take_profits)
        .unwrap();
    store
        .move_take_profits_of_level(&sell_working_level_id, distance_to_move_take_profits)
        .unwrap();

    store
        .get_working_level_chain_of_orders(&buy_working_level_id)
        .unwrap()
        .iter()
        .for_each(|order| {
            assert_eq!(
                order.props.base.prices.take_profit,
                take_profit - points_to_price(distance_to_move_take_profits)
            );
        });

    store
        .get_working_level_chain_of_orders(&sell_working_level_id)
        .unwrap()
        .iter()
        .for_each(|order| {
            assert_eq!(
                order.props.base.prices.take_profit,
                take_profit + points_to_price(distance_to_move_take_profits)
            );
        });
}

#[test]
fn should_return_error_when_inserting_nonexistent_entity() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(String::from("1")))
        .is_err());
    assert!(store
        .update_tendency_change_angle(String::from("1"))
        .is_err());
    assert!(store.update_min_angle(String::from("1")).is_err());
    assert!(store.update_virtual_min_angle(String::from("1")).is_err());
    assert!(store.update_max_angle(String::from("1")).is_err());
    assert!(store.update_virtual_max_angle(String::from("1")).is_err());
    assert!(store
        .update_min_angle_before_bargaining_corridor(String::from("1"))
        .is_err());
    assert!(store
        .update_max_angle_before_bargaining_corridor(String::from("1"))
        .is_err());
    assert!(store.update_current_tick(String::from("1")).is_err());
    assert!(store.update_previous_tick(String::from("1")).is_err());
    assert!(store.update_current_candle(String::from("1")).is_err());
    assert!(store.update_previous_candle(String::from("1")).is_err());
    assert!(store
        .update_max_crossing_value_of_working_level("1", dec!(10))
        .is_err());
    assert!(store
        .move_take_profits_of_level("1", dec!(0.00050))
        .is_err());
    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Small)
        .is_err());
}

#[test]
fn should_return_error_on_creating_entity_with_existing_id() {
    let mut store = InMemoryStepBacktestingStore::default();

    let tick_id = String::from("tick");
    store
        .create_tick(tick_id.clone(), Default::default())
        .unwrap();
    assert!(store.create_tick(tick_id, Default::default()).is_err());

    let candle_id = String::from("candle");
    store
        .create_candle(candle_id.clone(), Default::default())
        .unwrap();
    assert!(store
        .create_candle(candle_id.clone(), Default::default())
        .is_err());

    let angle_id = String::from("angle");
    store
        .create_angle(angle_id.clone(), Default::default(), candle_id.clone())
        .unwrap();
    assert!(store
        .create_angle(angle_id, Default::default(), candle_id)
        .is_err());

    let working_level_id = String::from("working_level");
    store
        .create_working_level(working_level_id.clone(), Default::default())
        .unwrap();
    assert!(store
        .create_working_level(working_level_id.clone(), Default::default())
        .is_err());

    let order_id = String::from("order");

    store
        .create_order(
            order_id.clone(),
            StepOrderProperties {
                working_level_id: working_level_id.clone(),
                ..Default::default()
            },
        )
        .unwrap();

    assert!(store
        .create_order(
            order_id,
            StepOrderProperties {
                working_level_id,
                ..Default::default()
            }
        )
        .is_err());
}
