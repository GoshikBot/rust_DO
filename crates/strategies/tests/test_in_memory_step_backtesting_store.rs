use std::collections::HashSet;

use base::entities::candle::CandleId;
use base::entities::order::OrderStatus;
use base::entities::tick::TickId;
use base::entities::Level;
use base::stores::candle_store::BasicCandleStore;
use base::stores::order_store::BasicOrderStore;
use base::stores::tick_store::BasicTickStore;
use strategies::step::utils::entities::angle::{AngleId, BasicAngleProperties};
use strategies::step::utils::entities::working_levels::CorridorType;
use strategies::step::utils::stores::angle_store::StepAngleStore;
use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use strategies::step::utils::stores::tick_store::StepTickStore;
use strategies::step::utils::stores::working_level_store::StepWorkingLevelStore;

#[test]
fn should_remove_only_unused_items() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let mut ticks = Vec::new();

    for _ in 1..=4 {
        let tick_id = store.create_tick(Default::default()).unwrap();
        ticks.push(tick_id);
    }

    assert!(store
        .update_current_tick(ticks.get(0).unwrap().clone())
        .is_ok());
    assert!(store
        .update_previous_tick(ticks.get(1).unwrap().clone())
        .is_ok());

    let mut candles = Vec::new();
    for _ in 1..=10 {
        let candle_id = store.create_candle(Default::default()).unwrap();
        candles.push(candle_id);
    }

    assert!(store
        .update_current_candle(candles.get(0).unwrap().clone())
        .is_ok());
    assert!(store
        .update_previous_candle(candles.get(1).unwrap().clone())
        .is_ok());

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candles.get(2).unwrap().clone(),
            CorridorType::Small,
        )
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candles.get(3).unwrap().clone(),
            CorridorType::Big,
        )
        .is_ok());

    let mut angles = Vec::new();

    for i in 1..=10 {
        let angle_id = store
            .create_angle(
                BasicAngleProperties { r#type: Level::Min },
                candles.get(i - 1).unwrap().clone(),
            )
            .unwrap();
        angles.push(angle_id);
    }

    assert!(store
        .update_angle_of_second_level_after_bargaining_tendency_change(
            angles.get(0).unwrap().clone()
        )
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

    assert!(store.remove_unused_items().is_ok());

    let left_ticks = ticks.drain(0..2).collect::<HashSet<_>>();

    assert!(store
        .get_all_ticks()
        .unwrap()
        .symmetric_difference(&left_ticks)
        .collect::<HashSet<&TickId>>()
        .is_empty());

    let left_candles = candles.drain(0..=7).collect::<HashSet<_>>();

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
        .is_empty())
}

#[test]
fn should_return_error_on_moving_working_level_to_active_if_it_is_not_present_in_created() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    assert!(store
        .move_working_level_to_active(&working_level_id)
        .is_ok());
    assert!(store
        .move_working_level_to_active(&working_level_id)
        .is_err());
}

#[test]
fn should_return_error_on_moving_working_level_to_removed_if_it_is_not_present_neither_in_created_nor_in_active(
) {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    assert!(store
        .move_working_level_to_removed(&working_level_id)
        .is_ok());
    assert!(store
        .move_working_level_to_removed(&working_level_id)
        .is_err());
}

#[test]
fn should_successfully_move_working_level_to_removed() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    for _ in 0..3 {
        let order_id = store.create_order(Default::default()).unwrap();
        store
            .add_order_to_working_level_chain_of_orders(&working_level_id, order_id)
            .unwrap();
    }

    assert!(store
        .move_working_level_to_removed(&working_level_id)
        .is_ok());

    assert!(store
        .get_removed_working_levels()
        .unwrap()
        .iter()
        .any(|level| level.id == working_level_id));

    for order in store
        .get_working_level_chain_of_orders(&working_level_id)
        .unwrap()
    {
        assert_eq!(order.props.base.status, OrderStatus::Closed);
    }
}

#[test]
fn should_successfully_remove_working_level() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    assert!(store
        .move_working_level_to_active(&working_level_id)
        .is_ok());

    assert!(store
        .update_max_crossing_value_of_working_level(&working_level_id, 10.0)
        .is_ok());

    let candle_id = store.create_candle(Default::default()).unwrap();

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candle_id.clone(),
            CorridorType::Small
        )
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor(
            &working_level_id,
            candle_id,
            CorridorType::Big
        )
        .is_ok());

    assert!(store.move_take_profits_of_level(&working_level_id).is_ok());

    let order_id = store.create_order(Default::default()).unwrap();

    assert!(store
        .add_order_to_working_level_chain_of_orders(&working_level_id, order_id)
        .is_ok());

    assert!(store.remove_working_level(&working_level_id).is_ok());

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

    assert!(!store
        .get_removed_working_levels()
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
        .are_take_profits_of_level_moved(&working_level_id)
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
fn should_successfully_add_candle_to_working_level_corridor() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    let first_candle_id = store.create_candle(Default::default()).unwrap();

    let second_candle_id = store.create_candle(Default::default()).unwrap();

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
}

#[test]
fn should_return_error_on_adding_candle_to_working_level_corridor_if_it_is_already_present_there() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    let candle_id = store.create_candle(Default::default()).unwrap();

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
fn should_successfully_add_order_to_working_level_chain_of_orders() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    let order_id = store.create_order(Default::default()).unwrap();

    assert!(store
        .add_order_to_working_level_chain_of_orders(&working_level_id, order_id.clone())
        .is_ok());

    assert!(store
        .get_working_level_chain_of_orders(&working_level_id)
        .unwrap()
        .iter()
        .any(|order| order.id == order_id));
}

#[test]
fn should_return_error_on_adding_order_to_working_level_chain_of_orders_if_it_is_already_present_there(
) {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    let working_level_id = store.create_working_level(Default::default()).unwrap();

    let order_id = store.create_order(Default::default()).unwrap();

    assert!(store
        .add_order_to_working_level_chain_of_orders(&working_level_id, order_id.clone())
        .is_ok());

    assert!(store
        .add_order_to_working_level_chain_of_orders(&working_level_id, order_id)
        .is_err());
}

#[test]
fn should_return_error_when_inserting_nonexistent_entity() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .update_angle_of_second_level_after_bargaining_tendency_change(String::from("1"))
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
        .update_max_crossing_value_of_working_level("1", 10.0)
        .is_err());
    assert!(store.move_take_profits_of_level("1").is_err());
    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Small)
        .is_err());
    assert!(store.move_take_profits_of_level("1").is_err());
}
