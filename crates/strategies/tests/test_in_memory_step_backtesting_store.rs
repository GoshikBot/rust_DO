use base::entities::candle::CandleId;
use base::entities::tick::TickId;
use base::entities::{CandleBaseProperties, Level, OrderType, TickBaseProperties};
use std::collections::HashSet;
use strategies::step::utils::entities::angles::{AngleBaseProperties, AngleId};
use strategies::step::utils::entities::working_levels::{CorridorType, WorkingLevelBaseProperties};
use strategies::step::utils::stores::base::{StepBacktestingStore, StepBaseStore};
use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;

#[test]
fn should_remove_only_unused_items() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    for i in 1..=4 {
        assert!(store.create_tick(i.to_string(), Default::default()).is_ok());
    }

    assert!(store.update_current_tick(String::from("1")).is_ok());
    assert!(store.update_previous_tick(String::from("2")).is_ok());

    for i in 1..=10 {
        assert!(store
            .create_candle(i.to_string(), Default::default(), Default::default())
            .is_ok());
    }

    assert!(store.update_current_candle(String::from("1")).is_ok());
    assert!(store.update_previous_candle(String::from("2")).is_ok());

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("3"), CorridorType::Small,)
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("4"), CorridorType::Big,)
        .is_ok());

    for i in 1..=10 {
        assert!(store
            .create_angle(
                i.to_string(),
                AngleBaseProperties {
                    candle_id: i.to_string(),
                    r#type: Level::Min,
                },
            )
            .is_ok());
    }

    assert!(store
        .update_angle_of_second_level_after_bargaining_tendency_change(String::from("1"))
        .is_ok());
    assert!(store
        .update_tendency_change_angle(String::from("2"))
        .is_ok());
    assert!(store.update_min_angle(String::from("3")).is_ok());
    assert!(store.update_max_angle(String::from("4")).is_ok());
    assert!(store.update_virtual_min_angle(String::from("5")).is_ok());
    assert!(store.update_virtual_max_angle(String::from("6")).is_ok());
    assert!(store
        .update_min_angle_before_bargaining_corridor(String::from("7"))
        .is_ok());
    assert!(store
        .update_max_angle_before_bargaining_corridor(String::from("8"))
        .is_ok());

    assert!(store.remove_unused_items().is_ok());

    let mut left_ticks = HashSet::new();
    left_ticks.insert(String::from("1"));
    left_ticks.insert(String::from("2"));

    assert!(store
        .get_all_ticks()
        .unwrap()
        .symmetric_difference(&left_ticks)
        .collect::<HashSet<&TickId>>()
        .is_empty());

    let mut left_candles = HashSet::new();
    left_candles.extend((1..=8).map(|i| i.to_string()));

    assert!(store
        .get_all_candles()
        .unwrap()
        .symmetric_difference(&left_candles)
        .collect::<HashSet<&CandleId>>()
        .is_empty());

    let mut left_angles = HashSet::new();
    left_angles.extend((1..=8).map(|i| i.to_string()));

    assert!(store
        .get_all_angles()
        .unwrap()
        .symmetric_difference(&left_angles)
        .collect::<HashSet<&AngleId>>()
        .is_empty())
}

#[test]
fn should_return_error_on_creating_angle_with_existing_id() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_angle(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_angle(String::from("1"), Default::default())
        .is_err());
}

#[test]
fn should_return_error_on_creating_tick_with_existing_id() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_tick(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_tick(String::from("1"), Default::default())
        .is_err());
}

#[test]
fn should_return_error_on_creating_candle_with_existing_id() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_candle(String::from("1"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .create_candle(String::from("1"), Default::default(), Default::default())
        .is_err())
}

#[test]
fn should_return_error_on_creating_working_level_with_existing_id() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_err());
}

#[test]
fn should_return_error_on_creating_order_with_existing_id() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_order(String::from("1"), Default::default(), Default::default())
        .is_ok());
    assert!(store
        .create_order(String::from("1"), Default::default(), Default::default())
        .is_err());
}

#[test]
fn should_return_error_on_moving_working_level_to_active_if_it_is_not_present_in_created() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store.move_working_level_to_active("1").is_ok());
    assert!(store.move_working_level_to_active("1").is_err());
}

#[test]
fn should_return_error_on_moving_working_level_to_removed_if_it_is_not_present_neither_in_created_nor_in_active(
) {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store.move_working_level_to_removed("1").is_ok());
    assert!(store.move_working_level_to_removed("1").is_err());
}

#[test]
fn should_successfully_remove_working_level() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store.move_working_level_to_active("1").is_ok());

    assert!(store
        .update_max_crossing_value_of_working_level("1", 10.0)
        .is_ok());

    assert!(store
        .create_candle(String::from("1"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Small)
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Big)
        .is_ok());

    assert!(store.move_take_profits_of_level("1").is_ok());

    assert!(store
        .create_order(String::from("1"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .add_order_to_working_level_chain_of_orders("1", String::from("1"))
        .is_ok());

    assert!(store.remove_working_level("1").is_ok());

    assert!(!store.get_created_working_levels().unwrap().contains("1"));
    assert!(!store.get_active_working_levels().unwrap().contains("1"));
    assert!(!store.get_removed_working_levels().unwrap().contains("1"));

    assert!(store
        .get_candles_of_working_level_corridor("1", CorridorType::Small)
        .unwrap()
        .is_none());

    assert!(store
        .get_candles_of_working_level_corridor("1", CorridorType::Big)
        .unwrap()
        .is_none());

    assert!(!store.are_take_profits_of_level_moved("1").unwrap());

    assert!(store
        .get_max_crossing_value_of_working_level("1")
        .unwrap()
        .is_none());

    assert!(store
        .get_working_level_chain_of_orders("1")
        .unwrap()
        .is_none());
}

#[test]
fn should_successfully_add_candle_to_working_level_corridor() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_candle(String::from("1"), Default::default(), Default::default())
        .is_ok());
    assert!(store
        .create_candle(String::from("2"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Small)
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("2"), CorridorType::Big)
        .is_ok());

    assert!(store
        .get_candles_of_working_level_corridor("1", CorridorType::Small)
        .unwrap()
        .unwrap()
        .contains("1"));

    assert!(store
        .get_candles_of_working_level_corridor("1", CorridorType::Big)
        .unwrap()
        .unwrap()
        .contains("2"));
}

#[test]
fn should_return_error_on_adding_candle_to_working_level_corridor_if_it_is_already_present_there() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_candle(String::from("1"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Small)
        .is_ok());
    assert!(store
        .add_candle_to_working_level_corridor("1", String::from("1"), CorridorType::Small)
        .is_err());
}

#[test]
fn should_successfully_add_order_to_working_level_chain_of_orders() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_order(String::from("1"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .add_order_to_working_level_chain_of_orders("1", String::from("1"))
        .is_ok());

    assert!(store
        .get_working_level_chain_of_orders("1")
        .unwrap()
        .unwrap()
        .contains("1"));
}

#[test]
fn should_return_error_on_adding_order_to_working_level_chain_of_orders_if_it_is_already_present_there(
) {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .create_working_level(String::from("1"), Default::default())
        .is_ok());

    assert!(store
        .create_order(String::from("1"), Default::default(), Default::default())
        .is_ok());

    assert!(store
        .add_order_to_working_level_chain_of_orders("1", String::from("1"))
        .is_ok());

    assert!(store
        .add_order_to_working_level_chain_of_orders("1", String::from("1"))
        .is_err());
}

#[test]
fn should_return_error_when_inserting_nonexistent_entity() {
    let mut store: InMemoryStepBacktestingStore = Default::default();

    assert!(store
        .update_candle_base_properties("1", Default::default())
        .is_err());
    assert!(store
        .update_working_level_base_properties("1", Default::default())
        .is_err());
    assert!(store
        .update_angle_base_properties("1", Default::default())
        .is_err());

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
