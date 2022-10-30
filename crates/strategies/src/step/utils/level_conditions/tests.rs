use super::*;
use crate::step::utils::entities::angle::{BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::entities::working_levels::{
    BacktestingWLProperties, WLId, WLMaxCrossingValue, WLStatus,
};
use crate::step::utils::stores::candle_store::StepCandleStore;
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use base::entities::candle::{BasicCandleProperties, CandleId, CandleVolatility};
use base::entities::order::OrderId;
use base::entities::tick::HistoricalTickPrice;
use base::entities::{CandlePrices, Item};
use base::stores::candle_store::BasicCandleStore;
use chrono::NaiveDate;
use rust_decimal_macros::dec;

#[test]
#[allow(non_snake_case)]
fn level_exceeds_amount_of_candles_in_corridor__len_of_small_corridor_is_greater_than_min_amount_of_candles__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let level = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap();

    for _ in 0..5 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(&level.id, candle.id, CorridorType::Small)
            .unwrap();
    }

    assert!(
        LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            &level.id,
            &store,
            CorridorType::Small,
            dec!(3),
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_exceeds_amount_of_candles_in_corridor__len_of_small_corridor_is_less_than_min_amount_of_candles__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let level = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap();

    for _ in 0..2 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(&level.id, candle.id, CorridorType::Small)
            .unwrap();
    }

    assert!(
        !LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            &level.id,
            &store,
            CorridorType::Small,
            dec!(3),
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_exceeds_amount_of_candles_in_corridor__len_of_big_corridor_is_greater_than_min_amount_of_candles__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let level = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap();

    for _ in 0..5 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(&level.id, candle.id, CorridorType::Big)
            .unwrap();
    }

    assert!(
        LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            &level.id,
            &store,
            CorridorType::Big,
            dec!(3),
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_exceeds_amount_of_candles_in_corridor__len_of_big_corridor_is_less_than_min_amount_of_candles__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let level = store
        .create_working_level(xid::new().to_string(), Default::default())
        .unwrap();

    for _ in 0..2 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(&level.id, candle.id, CorridorType::Big)
            .unwrap();
    }

    assert!(
        !LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            &level.id,
            &store,
            CorridorType::Big,
            dec!(3),
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_less_than_stop_loss_price__should_return_true(
) {
    assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Realtime(dec!(1.38500)),
        dec!(1.39000),
        OrderType::Buy
    ));

    assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Historical(HistoricalTickPrice {
            low: dec!(1.38500),
            high: dec!(1.39500),
            ..Default::default()
        }),
        dec!(1.39000),
        OrderType::Buy
    ));
}

#[test]
#[allow(non_snake_case)]
fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_greater_than_stop_loss_price__should_return_false(
) {
    assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Realtime(dec!(1.39500)),
        dec!(1.39000),
        OrderType::Buy
    ));

    assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Historical(HistoricalTickPrice {
            low: dec!(1.39500),
            high: dec!(1.40500),
            ..Default::default()
        }),
        dec!(1.39000),
        OrderType::Buy
    ));
}

#[test]
#[allow(non_snake_case)]
fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_greater_than_stop_loss_price__should_return_true(
) {
    assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Realtime(dec!(1.39500)),
        dec!(1.39000),
        OrderType::Sell
    ));

    assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Historical(HistoricalTickPrice {
            high: dec!(1.39500),
            low: dec!(1.38500),
            ..Default::default()
        }),
        dec!(1.39000),
        OrderType::Sell
    ));
}

#[test]
#[allow(non_snake_case)]
fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_less_than_stop_loss_price__should_return_false(
) {
    assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Realtime(dec!(1.38500)),
        dec!(1.39000),
        OrderType::Sell
    ));

    assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
        UniversalTickPrice::Historical(HistoricalTickPrice {
            high: dec!(1.38500),
            low: dec!(1.37500),
            ..Default::default()
        }),
        dec!(1.39000),
        OrderType::Sell
    ));
}

#[test]
#[allow(non_snake_case)]
fn level_expired_by_distance__current_tick_price_is_in_acceptable_range_from_level_price__should_return_false(
) {
    assert!(!LevelConditionsImpl::level_expired_by_distance(
        dec!(1.38000),
        UniversalTickPrice::Realtime(dec!(1.39000)),
        dec!(2_000)
    ));

    assert!(!LevelConditionsImpl::level_expired_by_distance(
        dec!(1.38000),
        UniversalTickPrice::Historical(HistoricalTickPrice {
            low: dec!(1.39000),
            high: dec!(1.37000),
            ..Default::default()
        }),
        dec!(2_000)
    ));
}

#[test]
#[allow(non_snake_case)]
fn level_expired_by_distance__current_tick_price_is_beyond_acceptable_range_from_level_price__should_return_true(
) {
    assert!(LevelConditionsImpl::level_expired_by_distance(
        dec!(1.38000),
        UniversalTickPrice::Realtime(dec!(1.40001)),
        dec!(2_000)
    ));

    assert!(LevelConditionsImpl::level_expired_by_distance(
        dec!(1.38000),
        UniversalTickPrice::Historical(HistoricalTickPrice {
            high: dec!(1.40001),
            low: dec!(1.37000),
            ..Default::default()
        }),
        dec!(2_000)
    ));

    assert!(LevelConditionsImpl::level_expired_by_distance(
        dec!(1.38000),
        UniversalTickPrice::Historical(HistoricalTickPrice {
            high: dec!(1.39000),
            low: dec!(1.35999),
            ..Default::default()
        }),
        dec!(2_000)
    ));
}

#[test]
#[allow(non_snake_case)]
fn level_expired_by_time__current_diff_is_greater_than_level_expiration__should_return_true() {
    let level_time = NaiveDate::from_ymd(2022, 8, 11).and_hms(0, 0, 0);
    let current_tick_time = NaiveDate::from_ymd(2022, 8, 19).and_hms(0, 0, 0);
    let level_expiration = dec!(5);

    let exclude_weekend_and_holidays =
        |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 2;

    assert!(LevelConditionsImpl::level_expired_by_time(
        level_time,
        current_tick_time,
        level_expiration,
        &exclude_weekend_and_holidays
    ));
}

#[test]
#[allow(non_snake_case)]
fn level_expired_by_time__current_diff_is_less_than_level_expiration__should_return_false() {
    let level_time = NaiveDate::from_ymd(2022, 8, 11).and_hms(0, 0, 0);
    let current_tick_time = NaiveDate::from_ymd(2022, 8, 19).and_hms(0, 0, 0);
    let level_expiration = dec!(7);

    let exclude_weekend_and_holidays =
        |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 2;

    assert!(!LevelConditionsImpl::level_expired_by_time(
        level_time,
        current_tick_time,
        level_expiration,
        &exclude_weekend_and_holidays
    ));
}

#[test]
#[allow(non_snake_case)]
fn level_has_no_opened_orders__all_orders_are_pending__should_return_true() {
    let orders = vec![
        BasicOrderProperties::default(),
        BasicOrderProperties::default(),
        BasicOrderProperties::default(),
        BasicOrderProperties::default(),
    ];

    assert!(LevelConditionsImpl::level_has_no_active_orders(&orders));
}

#[test]
#[allow(non_snake_case)]
fn level_has_no_opened_orders__some_orders_are_opened__should_return_false() {
    let orders = vec![
        BasicOrderProperties::default(),
        BasicOrderProperties::default(),
        BasicOrderProperties {
            status: OrderStatus::Opened,
            ..Default::default()
        },
        BasicOrderProperties::default(),
    ];

    assert!(!LevelConditionsImpl::level_has_no_active_orders(&orders));
}

#[test]
#[allow(non_snake_case)]
fn level_has_no_opened_orders__some_orders_are_closed__should_return_false() {
    let orders = vec![
        BasicOrderProperties::default(),
        BasicOrderProperties::default(),
        BasicOrderProperties {
            status: OrderStatus::Closed,
            ..Default::default()
        },
        BasicOrderProperties::default(),
    ];

    assert!(!LevelConditionsImpl::level_has_no_active_orders(&orders));
}

#[test]
#[allow(non_snake_case)]
fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_buy_level_max_crossing_value_is_beyond_limit__should_return_true(
) {
    let level = BasicWLProperties {
        price: dec!(1.38000),
        r#type: OrderType::Buy,
        ..Default::default()
    };

    let max_crossing_value = dec!(200);
    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(1.38050));
    let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
        dec!(100);

    assert!(
        LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            realtime_current_tick_price,
        ));

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.38050),
        low: dec!(1.37950),
        ..Default::default()
    });

    assert!(
        LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            historical_current_tick_price,
        ));
}

#[test]
#[allow(non_snake_case)]
fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__has_not_returned_to_buy_level_max_crossing_value_is_beyond_limit__should_return_false(
) {
    let level = BasicWLProperties {
        price: dec!(1.38000),
        r#type: OrderType::Buy,
        ..Default::default()
    };

    let max_crossing_value = dec!(200);
    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(1.37999));
    let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
        dec!(100);

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        realtime_current_tick_price,
    ));

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.37999),
        low: dec!(1.37950),
        ..Default::default()
    });

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        historical_current_tick_price,
    ));
}

#[test]
#[allow(non_snake_case)]
fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_buy_level_max_crossing_value_is_not_beyond_limit__should_return_false(
) {
    let level = BasicWLProperties {
        price: dec!(1.38000),
        r#type: OrderType::Buy,
        ..Default::default()
    };

    let max_crossing_value = dec!(99);
    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(1.38050));
    let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
        dec!(100);

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        realtime_current_tick_price,
    ));

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.38050),
        low: dec!(1.37950),
        ..Default::default()
    });

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        historical_current_tick_price,
    ));
}

#[test]
#[allow(non_snake_case)]
fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_sell_level_max_crossing_value_is_beyond_limit__should_return_true(
) {
    let level = BasicWLProperties {
        price: dec!(1.38000),
        r#type: OrderType::Sell,
        ..Default::default()
    };

    let max_crossing_value = dec!(200);
    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(1.37999));
    let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
        dec!(100);

    assert!(LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        realtime_current_tick_price,
    ));

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.38050),
        low: dec!(1.37999),
        ..Default::default()
    });

    assert!(LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        historical_current_tick_price,
    ));
}

#[test]
#[allow(non_snake_case)]
fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_sell_level_max_crossing_value_is_not_beyond_limit__should_return_false(
) {
    let level = BasicWLProperties {
        price: dec!(1.38000),
        r#type: OrderType::Sell,
        ..Default::default()
    };

    let max_crossing_value = dec!(50);
    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(1.37999));
    let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
        dec!(100);

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        realtime_current_tick_price,
    ));

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.38050),
        low: dec!(1.37999),
        ..Default::default()
    });

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        historical_current_tick_price,
    ));
}

#[test]
#[allow(non_snake_case)]
fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__has_not_returned_to_sell_level_max_crossing_value_is_beyond_limit__should_return_false(
) {
    let level = BasicWLProperties {
        price: dec!(1.38000),
        r#type: OrderType::Sell,
        ..Default::default()
    };

    let max_crossing_value = dec!(200);
    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(1.38001));
    let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
        dec!(100);

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        realtime_current_tick_price,
    ));

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.38050),
        low: dec!(1.38001),
        ..Default::default()
    });

    assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        &level,
        Some(max_crossing_value),
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
        historical_current_tick_price,
    ));
}

#[test]
#[allow(non_snake_case)]
fn is_second_level_after_bargaining_tendency_change__tendency_change_angle_is_none__should_return_false(
) {
    let crossed_angle = "1";

    let tendency_change_angle = None;

    assert!(
        !LevelConditionsImpl::is_second_level_after_bargaining_tendency_change(
            crossed_angle,
            tendency_change_angle,
            true,
            false
        )
    );
}

#[test]
#[allow(non_snake_case)]
fn is_second_level_after_bargaining_tendency_change__tendency_change_angle_exists_and_last_tendency_changed_on_crossing_bargaining_corridor_and_second_level_after_bargaining_tendency_change_is_not_created_and_crossed_angle_is_not_tendency_change_angle__should_return_true(
) {
    let crossed_angle = "1";

    let tendency_change_angle = Some("2");

    assert!(
        LevelConditionsImpl::is_second_level_after_bargaining_tendency_change(
            crossed_angle,
            tendency_change_angle,
            true,
            false
        )
    );
}

#[test]
#[allow(non_snake_case)]
fn is_second_level_after_bargaining_tendency_change__tendency_change_angle_exists_and_last_tendency_has_not_changed_on_crossing_bargaining_corridor_and_second_level_after_bargaining_tendency_change_is_not_created_and_crossed_angle_is_not_tendency_change_angle__should_return_false(
) {
    let crossed_angle = "1";

    let tendency_change_angle = Some("2");

    assert!(
        !LevelConditionsImpl::is_second_level_after_bargaining_tendency_change(
            crossed_angle,
            tendency_change_angle,
            false,
            false
        )
    );
}

#[test]
#[allow(non_snake_case)]
fn is_second_level_after_bargaining_tendency_change__tendency_change_angle_exists_and_last_tendency_changed_on_crossing_bargaining_corridor_and_second_level_after_bargaining_tendency_change_is_created_and_crossed_angle_is_not_tendency_change_angle__should_return_false(
) {
    let crossed_angle = "1";

    let tendency_change_angle = Some("2");

    assert!(
        !LevelConditionsImpl::is_second_level_after_bargaining_tendency_change(
            crossed_angle,
            tendency_change_angle,
            true,
            true
        )
    );
}

#[test]
#[allow(non_snake_case)]
fn is_second_level_after_bargaining_tendency_change__tendency_change_angle_exists_and_last_tendency_changed_on_crossing_bargaining_corridor_and_second_level_after_bargaining_tendency_change_is_not_created_and_crossed_angle_is_tendency_change_angle__should_return_false(
) {
    let crossed_angle = "1";

    let tendency_change_angle = Some("1");

    assert!(
        !LevelConditionsImpl::is_second_level_after_bargaining_tendency_change(
            crossed_angle,
            tendency_change_angle,
            true,
            false
        )
    );
}

// level_comes_out_of_bargaining_corridor cases to test:
// - len of bargaining corridor is less than min len (should return false)
// - len of bargaining corridor is more than min len && min angle is None && max angle exists
//   (should return false)
// - len of bargaining corridor is more than min len && min angle exists && max angle is None
//   (should return false)
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains min angle and not max angle (should return false)
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains max angle and not min angle (should return false)
//
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains both max and min angles && crossed angle type is min &&
//   max angle before bargaining corridor is None (should return false)
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains both max and min angles && crossed angle type is min &&
//   max angle before bargaining corridor exists && max angle before bargaining
//   corridor >= current max angle (should return false)
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains both max and min angles && crossed angle type is min &&
//   max angle before bargaining corridor exists && max angle before bargaining
//   corridor < current max angle (should return true)
//
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains both max and min angles && crossed angle type is max &&
//   min angle before bargaining corridor is None (should return false)
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains both max and min angles && crossed angle type is max &&
//   min angle before bargaining corridor exists && min angle before bargaining
//   corridor <= current min angle (should return false)
// - len of bargaining corridor is more than min len && both min and max angles exist &&
//   general corridor contains both max and min angles && crossed angle type is max &&
//   min angle before bargaining corridor exists && min angle before bargaining
//   corridor > current min angle (should return true)

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_less_than_min_len__should_return_false(
) {
    let store = InMemoryStepBacktestingStore::default();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties::default(),
        },
    };

    let general_corridor = Vec::new();

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(1);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_min_angle_is_none_and_max_angle_exists__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            candle.id,
        )
        .unwrap();

    store.update_max_angle(angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties::default(),
        },
    };

    let general_corridor = vec![
        Item {
            id: String::from("1"),
            props: StepBacktestingCandleProperties::default(),
        },
        Item {
            id: String::from("2"),
            props: StepBacktestingCandleProperties::default(),
        },
    ];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_min_angle_exists_and_max_angle_is_none__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            candle.id,
        )
        .unwrap();

    store.update_min_angle(angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties::default(),
        },
    };

    let general_corridor = vec![
        Item {
            id: String::from("1"),
            props: StepBacktestingCandleProperties::default(),
        },
        Item {
            id: String::from("2"),
            props: StepBacktestingCandleProperties::default(),
        },
    ];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_min_and_not_max_angle__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties::default(),
        },
    };

    let general_corridor = vec![
        Item {
            id: String::from("1"),
            props: StepBacktestingCandleProperties::default(),
        },
        min_angle_candle,
    ];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_max_and_not_min_angle__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties::default(),
        },
    };

    let general_corridor = vec![
        Item {
            id: String::from("1"),
            props: StepBacktestingCandleProperties::default(),
        },
        max_angle_candle,
    ];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_both_min_and_max_angles_and_crossed_angle_type_is_min_and_max_angle_before_bargaining_corridor_is_none__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
        },
    };

    let general_corridor = vec![min_angle_candle, max_angle_candle];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_both_min_and_max_angles_and_crossed_angle_type_is_min_and_max_angle_before_bargaining_corridor_exists_and_max_angle_before_bargaining_corridor_is_greater_than_current_max_angle__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let max_angle_before_bargaining_corridor_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle_before_bargaining_corridor = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_before_bargaining_corridor_candle.id.clone(),
        )
        .unwrap();

    store
        .update_max_angle_before_bargaining_corridor(max_angle_before_bargaining_corridor.id)
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.37000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
        },
    };

    let general_corridor = vec![min_angle_candle, max_angle_candle];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_both_min_and_max_angles_and_crossed_angle_type_is_min_and_max_angle_before_bargaining_corridor_exists_and_max_angle_before_bargaining_corridor_is_less_than_current_max_angle__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let max_angle_before_bargaining_corridor_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle_before_bargaining_corridor = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_before_bargaining_corridor_candle.id.clone(),
        )
        .unwrap();

    store
        .update_max_angle_before_bargaining_corridor(max_angle_before_bargaining_corridor.id)
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.39000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
        },
    };

    let general_corridor = vec![min_angle_candle, max_angle_candle];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
        &crossed_angle,
        &general_corridor,
        &store,
        min_amount_of_candles_in_corridor_defining_edge_bargaining,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_both_min_and_max_angles_and_crossed_angle_type_is_max_and_min_angle_before_bargaining_corridor_is_none__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties::default(),
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
        },
    };

    let general_corridor = vec![min_angle_candle, max_angle_candle];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_both_min_and_max_angles_and_crossed_angle_type_is_max_and_min_angle_before_bargaining_corridor_exists_and_min_angle_before_bargaining_corridor_is_less_than_current_min_angle__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.37000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let min_angle_before_bargaining_corridor_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.36000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle_before_bargaining_corridor = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_before_bargaining_corridor_candle.id.clone(),
        )
        .unwrap();

    store
        .update_min_angle_before_bargaining_corridor(min_angle_before_bargaining_corridor.id)
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
        },
    };

    let general_corridor = vec![min_angle_candle, max_angle_candle];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(
        !LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
            &crossed_angle,
            &general_corridor,
            &store,
            min_amount_of_candles_in_corridor_defining_edge_bargaining,
        )
        .unwrap()
    );
}

#[test]
#[allow(non_snake_case)]
fn level_comes_out_of_bargaining_corridor__len_of_bargaining_corridor_is_more_than_min_len_and_both_min_and_max_angle_exist_and_general_corridor_contains_both_min_and_max_angles_and_crossed_angle_type_is_max_and_min_angle_before_bargaining_corridor_exists_and_min_angle_before_bargaining_corridor_is_greater_than_current_min_angle__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.37000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id.clone(),
        )
        .unwrap();

    let min_angle_before_bargaining_corridor_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle_before_bargaining_corridor = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_before_bargaining_corridor_candle.id.clone(),
        )
        .unwrap();

    store
        .update_min_angle_before_bargaining_corridor(min_angle_before_bargaining_corridor.id)
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
        },
    };

    let general_corridor = vec![min_angle_candle, max_angle_candle];

    let min_amount_of_candles_in_corridor_defining_edge_bargaining = dec!(2);

    assert!(LevelConditionsImpl::level_comes_out_of_bargaining_corridor(
        &crossed_angle,
        &general_corridor,
        &store,
        min_amount_of_candles_in_corridor_defining_edge_bargaining,
    )
    .unwrap());
}

// appropriate_working_level cases to test
// - min angle is None (should return false)
// - max angle is None (should return false)
//
// - min crossed angle && min break distance is NOT present (should return false)
// - min crossed angle && min break distance is present && max angle time is later than
//   min angle time (should return true)
// - min crossed angle && min break distance is present by gap && max angle time is later than
//   min angle time (should return true)
// - min crossed angle && min break distance is present && max angle time is earlier than
//   min angle time && virtual max angle time is later than min angle time (should return true)
// - min crossed angle && min break distance is present && max angle time is earlier than
//   min angle time && virtual max angle time is earlier than min angle time &&
//   min distance between current candle high and min angle is present (should return true)
// - min crossed angle && min break distance is present && max angle time is earlier than
//   min angle time && virtual max angle is None && min distance between current candle high
//   and min angle is present (should return true)
// - min crossed angle && min break distance is present && max angle time is earlier than
//   min angle time && virtual max angle is None && min distance between current candle high
//   and min angle is NOT present (should return false)
//
// - max crossed angle && min break distance is NOT present (should return false)
// - max crossed angle && min break distance is present && min angle time is later than
//   max angle time (should return true)
// - max crossed angle && min break distance is present by gap && min angle time is later than
//   max angle time (should return true)
// - max crossed angle && min break distance is present && min angle time is earlier than
//   max angle time && virtual min angle time is later than max angle time (should return true)
// - max crossed angle && min break distance is present && min angle time is earlier than
//   max angle time && virtual min angle time is earlier than max angle time &&
//   min distance between max angle and current candle low is present (should return true)
// - max crossed angle && min break distance is present && min angle time is earlier than
//   max angle time && virtual min angle is None && min distance between max angle and
//   current candle low is present (should return true)
// - max crossed angle && min break distance is present && min angle time is earlier than
//   max angle time && virtual min angle is None && min distance between max angle and
//   current candle low is NOT present (should return false)

#[derive(Default)]
struct AppropriateWorkingLevelTestParams;

impl StrategyParams for AppropriateWorkingLevelTestParams {
    type PointParam = StepPointParam;
    type RatioParam = StepRatioParam;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamOutputValue {
        unimplemented!()
    }

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        volatility: CandleVolatility,
    ) -> ParamOutputValue {
        match name {
            StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles => dec!(100),
            StepRatioParam::MinBreakDistance => dec!(30),
            _ => unimplemented!(),
        }
    }
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__min_angle_is_none_and_max_angle_exists__should_return_false() {
    let mut store = InMemoryStepBacktestingStore::default();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
        },
    };

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties::default(),
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(!LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__min_angle_exists_and_max_angle_is_none__should_return_false() {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
        },
    };

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties::default(),
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(!LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_not_present__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.39000),
                        close: dec!(1.38001),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(!LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_present_and_max_angle_time_is_later_than_min_angle_time__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 3).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.39000),
                        close: dec!(1.37970),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_present_by_gap_and_max_angle_time_is_later_than_min_angle_time__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 3).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.37970),
                        close: dec!(1.38100),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_present_and_max_angle_time_is_earlier_than_min_angle_time_and_virtual_max_angle_time_is_later_than_min_angle_time__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    let virtual_max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 6).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let virtual_max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            virtual_max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();
    store
        .update_virtual_max_angle(virtual_max_angle.id)
        .unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.39000),
                        close: dec!(1.37970),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_present_and_max_angle_time_is_earlier_than_min_angle_time_and_virtual_max_angle_time_is_earlier_than_min_angle_time_and_min_distance_between_current_candle_high_and_min_angle_is_present__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    let virtual_max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let virtual_max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            virtual_max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();
    store
        .update_virtual_max_angle(virtual_max_angle.id)
        .unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.38070),
                        high: dec!(1.38100),
                        close: dec!(1.37970),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_present_and_max_angle_time_is_earlier_than_min_angle_time_and_virtual_max_angle_is_none_and_min_distance_between_current_candle_high_and_min_angle_is_present__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.38070),
                        high: dec!(1.38100),
                        close: dec!(1.37970),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_min_crossed_angle_and_min_break_distance_is_present_and_max_angle_time_is_earlier_than_min_angle_time_and_virtual_max_angle_is_none_and_min_distance_between_current_candle_high_and_min_angle_is_not_present__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id.clone()).unwrap();
    store.update_max_angle(max_angle.id).unwrap();

    let crossed_angle = min_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.38070),
                        high: dec!(1.38099),
                        close: dec!(1.37970),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(!LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_not_present__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.37900),
                        close: dec!(1.38029),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(!LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_present_and_min_angle_time_is_later_than_max_angle_time__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 3).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.38900),
                        close: dec!(1.38030),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_present_by_gap_and_min_angle_time_is_later_than_max_angle_time__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 3).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.38030),
                        close: dec!(1.37900),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_present_and_min_angle_time_is_earlier_than_max_angle_time_and_virtual_min_angle_time_is_later_than_max_angle_time__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    let virtual_min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 6).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let virtual_min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            virtual_min_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();
    store
        .update_virtual_min_angle(virtual_min_angle.id)
        .unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.37900),
                        close: dec!(1.38030),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_present_and_min_angle_time_is_earlier_than_max_angle_time_and_virtual_min_angle_time_is_earlier_than_max_angle_time_and_min_distance_between_max_angle_and_current_candle_low_is_present__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    let virtual_min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let virtual_min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            virtual_min_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();
    store
        .update_virtual_min_angle(virtual_min_angle.id)
        .unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.37950),
                        low: dec!(1.37900),
                        close: dec!(1.38030),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_present_and_min_angle_time_is_earlier_than_max_angle_time_and_virtual_min_is_none_and_min_distance_between_max_angle_and_current_candle_low_is_present__should_return_true(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.37950),
                        low: dec!(1.37900),
                        close: dec!(1.38030),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn appropriate_working_level__both_min_and_max_angles_exist_and_max_crossed_angle_and_min_break_distance_is_present_and_min_angle_time_is_earlier_than_max_angle_time_and_virtual_min_is_none_and_min_distance_between_max_angle_and_current_candle_low_is_not_present__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let min_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let min_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_candle.id,
        )
        .unwrap();

    let max_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        time: NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let max_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(min_angle.id).unwrap();
    store.update_max_angle(max_angle.id.clone()).unwrap();

    let crossed_angle = max_angle;

    let current_candle = Item {
        id: String::from("1"),
        props: StepBacktestingCandleProperties {
            step_common: StepCandleProperties {
                base: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(1.37950),
                        low: dec!(1.37901),
                        close: dec!(1.38030),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    };

    let params = AppropriateWorkingLevelTestParams::default();

    assert!(!LevelConditionsImpl::appropriate_working_level(
        &crossed_angle,
        &current_candle,
        &store,
        &params,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_exists__level_is_present_in_created_working_levels__should_return_true() {
    let mut store = InMemoryStepBacktestingStore::default();

    let price = dec!(1.38000);
    let time = NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0);

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price,
                    time,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        time,
                        ..Default::default()
                    },
                    leading_price: price,
                },
            },
            base: BasicAngleProperties::default(),
        },
    };

    assert!(LevelConditionsImpl::working_level_exists(&crossed_angle, &store).unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_exists__level_is_present_in_active_working_levels__should_return_true() {
    let mut store = InMemoryStepBacktestingStore::default();

    let price = dec!(1.38000);
    let time = NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0);

    let level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price,
                    time,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        time,
                        ..Default::default()
                    },
                    leading_price: price,
                },
            },
            base: BasicAngleProperties::default(),
        },
    };

    assert!(LevelConditionsImpl::working_level_exists(&crossed_angle, &store).unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_exists__level_is_not_present_neither_in_active_nor_in_active_working_levels__should_return_true(
) {
    let store = InMemoryStepBacktestingStore::default();

    let price = dec!(1.38000);
    let time = NaiveDate::from_ymd(2022, 4, 5).and_hms(0, 0, 0);

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        time,
                        ..Default::default()
                    },
                    leading_price: price,
                },
            },
            base: BasicAngleProperties::default(),
        },
    };

    assert!(!LevelConditionsImpl::working_level_exists(&crossed_angle, &store).unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_is_close_to_another_one__close_to_existing_created_buy_level__should_return_true()
{
    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price: dec!(1.37900),
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
            },
        },
    };

    let distance_defining_nearby_levels_of_the_same_type = dec!(100);

    assert!(LevelConditionsImpl::working_level_is_close_to_another_one(
        &crossed_angle,
        &store,
        distance_defining_nearby_levels_of_the_same_type,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_is_close_to_another_one__far_from_existing_created_buy_level__should_return_false()
{
    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price: dec!(1.37899),
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
            },
        },
    };

    let distance_defining_nearby_levels_of_the_same_type = dec!(100);

    assert!(!LevelConditionsImpl::working_level_is_close_to_another_one(
        &crossed_angle,
        &store,
        distance_defining_nearby_levels_of_the_same_type,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_is_close_to_another_one__opposite_from_existing_created_buy_level__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price: dec!(1.38100),
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            high: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
            },
        },
    };

    let distance_defining_nearby_levels_of_the_same_type = dec!(100);

    assert!(!LevelConditionsImpl::working_level_is_close_to_another_one(
        &crossed_angle,
        &store,
        distance_defining_nearby_levels_of_the_same_type,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_is_close_to_another_one__close_to_existing_active_sell_level__should_return_true()
{
    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price: dec!(1.38100),
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
            },
        },
    };

    let distance_defining_nearby_levels_of_the_same_type = dec!(100);

    assert!(LevelConditionsImpl::working_level_is_close_to_another_one(
        &crossed_angle,
        &store,
        distance_defining_nearby_levels_of_the_same_type,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_is_close_to_another_one__far_from_existing_active_sell_level__should_return_false()
{
    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price: dec!(1.38101),
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
            },
        },
    };

    let distance_defining_nearby_levels_of_the_same_type = dec!(100);

    assert!(!LevelConditionsImpl::working_level_is_close_to_another_one(
        &crossed_angle,
        &store,
        distance_defining_nearby_levels_of_the_same_type,
    )
    .unwrap());
}

#[test]
#[allow(non_snake_case)]
fn working_level_is_close_to_another_one__opposite_from_existing_active_sell_level__should_return_false(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    price: dec!(1.37900),
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepCandleProperties {
                    base: BasicCandleProperties {
                        prices: CandlePrices {
                            low: dec!(1.38000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    leading_price: dec!(1.38000),
                },
            },
        },
    };

    let distance_defining_nearby_levels_of_the_same_type = dec!(100);

    assert!(!LevelConditionsImpl::working_level_is_close_to_another_one(
        &crossed_angle,
        &store,
        distance_defining_nearby_levels_of_the_same_type,
    )
    .unwrap());
}
