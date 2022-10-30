use crate::step::utils::entities::working_levels::{
    BacktestingWLProperties, CorridorType, LevelTime, WLPrice,
};
use crate::step::utils::entities::FakeBacktestingNotificationQueue;
use crate::step::utils::level_conditions::{LevelConditionsImpl, MinAmountOfCandles};
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use crate::step::utils::stores::StepBacktestingStatistics;
use base::entities::candle::CandleId;
use base::entities::order::{BasicOrderPrices, BasicOrderProperties, OrderPrice, OrderStatus};
use base::entities::tick::{HistoricalTickPrice, TickTime};
use base::helpers::points_to_price;
use base::notifier::Message;
use base::params::ParamOutputValue;
use base::stores::candle_store::BasicCandleStore;
use base::stores::order_store::BasicOrderStore;
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cell::RefCell;
use std::env;

use super::*;

#[test]
#[allow(non_snake_case)]
fn get_crossed_level__current_tick_price_is_less_than_buy_level_price__should_return_buy_level() {
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

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(9),
        low: dec!(8),
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(8));

    let crossed_level =
        LevelUtilsImpl::get_crossed_level(historical_current_tick_price, &created_working_levels);

    assert_eq!(crossed_level.unwrap().id, "1");

    let crossed_level =
        LevelUtilsImpl::get_crossed_level(realtime_current_tick_price, &created_working_levels);

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

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(12),
        low: dec!(11),
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(12));

    let crossed_level =
        LevelUtilsImpl::get_crossed_level(historical_current_tick_price, &created_working_levels);

    assert_eq!(crossed_level.unwrap().id, "2");

    let crossed_level =
        LevelUtilsImpl::get_crossed_level(realtime_current_tick_price, &created_working_levels);

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

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(11.5),
        low: dec!(11),
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(dec!(11));
    let current_tick_price = dec!(11);

    let crossed_level =
        LevelUtilsImpl::get_crossed_level(historical_current_tick_price, &created_working_levels);

    assert!(crossed_level.is_none());

    let crossed_level =
        LevelUtilsImpl::get_crossed_level(realtime_current_tick_price, &created_working_levels);

    assert!(crossed_level.is_none());
}

#[test]
#[allow(non_snake_case)]
fn remove_active_working_levels_with_closed_orders__two_active_working_levels_with_closed_orders_exist__should_remove_these_two_levels(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let mut working_level_ids = Vec::new();

    for _ in 0..4 {
        working_level_ids.push(
            store
                .create_working_level(xid::new().to_string(), Default::default())
                .unwrap()
                .id,
        );
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
                .create_order(
                    xid::new().to_string(),
                    StepOrderProperties {
                        base: BasicOrderProperties {
                            status,
                            ..Default::default()
                        },
                        working_level_id: working_level_ids[0].clone(),
                    },
                )
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
                .create_order(
                    xid::new().to_string(),
                    StepOrderProperties {
                        base: BasicOrderProperties {
                            status,
                            ..Default::default()
                        },
                        working_level_id: working_level_ids[2].clone(),
                    },
                )
                .unwrap()
                .id
        })
        .collect();

    let first_chain_of_orders_without_closed_orders: Vec<_> = (0..5)
        .into_iter()
        .map(|_| {
            store
                .create_order(
                    xid::new().to_string(),
                    StepOrderProperties {
                        working_level_id: working_level_ids[1].clone(),
                        ..Default::default()
                    },
                )
                .unwrap()
                .id
        })
        .collect();

    let second_chain_of_orders_without_closed_orders: Vec<_> = (0..5)
        .into_iter()
        .map(|_| {
            store
                .create_order(
                    xid::new().to_string(),
                    StepOrderProperties {
                        working_level_id: working_level_ids[3].clone(),
                        ..Default::default()
                    },
                )
                .unwrap()
                .id
        })
        .collect();

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

    let level_props = BacktestingWLProperties {
        base: BasicWLProperties {
            r#type: OrderType::Buy,
            price: level_price,
            ..Default::default()
        },
        chart_index: 0,
    };

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let main_price = dec!(1.37000);

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.39000),
        low: main_price,
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(main_price);

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    let expected_max_crossing_value = price_to_points(level_price - main_price);

    assert_eq!(
        store
            .get_max_crossing_value_of_working_level(&level.id)
            .unwrap()
            .unwrap(),
        expected_max_crossing_value
    );

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        realtime_current_tick_price,
    )
    .unwrap();

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

    let level_props = BacktestingWLProperties {
        base: BasicWLProperties {
            r#type: OrderType::Sell,
            price: level_price,
            ..Default::default()
        },
        chart_index: 0,
    };

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let main_price = dec!(1.39000);

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: main_price,
        low: dec!(1.37000),
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(main_price);

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    let expected_max_crossing_value = price_to_points(main_price - level_price);

    assert_eq!(
        store
            .get_max_crossing_value_of_working_level(&level.id)
            .unwrap()
            .unwrap(),
        expected_max_crossing_value
    );

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        realtime_current_tick_price,
    )
    .unwrap();

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

    let level_props = BacktestingWLProperties {
        base: BasicWLProperties {
            r#type: OrderType::Buy,
            price: level_price,
            ..Default::default()
        },
        chart_index: 0,
    };

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let main_price = dec!(1.39000);

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.39500),
        low: main_price,
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(main_price);

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    assert!(store
        .get_max_crossing_value_of_working_level(&level.id)
        .unwrap()
        .is_none());

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    assert!(store
        .get_max_crossing_value_of_working_level(&level.id)
        .unwrap()
        .is_none());

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        realtime_current_tick_price,
    )
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

    let level_props = BacktestingWLProperties {
        base: BasicWLProperties {
            r#type: OrderType::Sell,
            price: level_price,
            ..Default::default()
        },
        chart_index: 0,
    };

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let main_price = dec!(1.37000);

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: main_price,
        low: dec!(1.36000),
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(main_price);

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    assert!(store
        .get_max_crossing_value_of_working_level(&level.id)
        .unwrap()
        .is_none());

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        realtime_current_tick_price,
    )
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

    let level_props = BacktestingWLProperties {
        base: BasicWLProperties {
            r#type: OrderType::Buy,
            price: level_price,
            ..Default::default()
        },
        chart_index: 0,
    };

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let previous_max_crossing_value = dec!(200);

    store
        .update_max_crossing_value_of_working_level(&level.id, previous_max_crossing_value)
        .unwrap();

    let main_price = dec!(1.37000);

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.39000),
        low: main_price,
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(main_price);

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    let expected_max_crossing_value = price_to_points(level_price - main_price);

    assert_eq!(
        store
            .get_max_crossing_value_of_working_level(&level.id)
            .unwrap()
            .unwrap(),
        expected_max_crossing_value
    );

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store
        .update_max_crossing_value_of_working_level(&level.id, previous_max_crossing_value)
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        realtime_current_tick_price,
    )
    .unwrap();

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
fn update_max_crossing_value_of_level__crossing_value_is_less_than_previous__should_not_update_crossing_value(
) {
    let mut store = InMemoryStepBacktestingStore::new();

    let level_price = dec!(1.38000);

    let level_props = BacktestingWLProperties {
        base: BasicWLProperties {
            r#type: OrderType::Buy,
            price: level_price,
            ..Default::default()
        },
        chart_index: 0,
    };

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store.move_working_level_to_active(&level.id).unwrap();

    let previous_max_crossing_value = dec!(2000);

    store
        .update_max_crossing_value_of_working_level(&level.id, previous_max_crossing_value)
        .unwrap();

    let main_price = dec!(1.37000);

    let historical_current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.39000),
        low: main_price,
        ..Default::default()
    });

    let realtime_current_tick_price = UniversalTickPrice::Realtime(main_price);

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        historical_current_tick_price,
    )
    .unwrap();

    assert_eq!(
        store
            .get_max_crossing_value_of_working_level(&level.id)
            .unwrap()
            .unwrap(),
        previous_max_crossing_value
    );

    let level = store
        .create_working_level(xid::new().to_string(), level_props.clone())
        .unwrap();

    store
        .update_max_crossing_value_of_working_level(&level.id, previous_max_crossing_value)
        .unwrap();

    LevelUtilsImpl::update_max_crossing_value_of_working_levels(
        &mut store,
        realtime_current_tick_price,
    )
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
        unimplemented!()
    }

    fn price_is_beyond_stop_loss(
        _current_tick_price: UniversalTickPrice,
        _stop_loss_price: OrderPrice,
        _working_level_type: OrderType,
    ) -> bool {
        unimplemented!()
    }

    fn level_expired_by_distance(
        level_price: WLPrice,
        _current_tick_price: UniversalTickPrice,
        _distance_from_level_for_its_deletion: ParamOutputValue,
    ) -> bool {
        level_price == dec!(1) || level_price == dec!(5)
    }

    fn level_expired_by_time(
        level_time: LevelTime,
        _current_tick_time: TickTime,
        _level_expiration: ParamOutputValue,
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
        _min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamOutputValue,
        _current_tick_price: UniversalTickPrice,
    ) -> bool {
        level.as_ref().price == dec!(7)
    }

    fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
        level_orders.is_empty()
    }

    fn is_second_level_after_bargaining_tendency_change(
        crossed_angle: &str,
        tendency_change_angle: Option<&str>,
        last_tendency_changed_on_crossing_bargaining_corridor: bool,
        second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        unimplemented!()
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        general_corridor: &[Item<CandleId, C>],
        angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        unimplemented!()
    }

    fn appropriate_working_level<A, C>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        current_candle: &Item<CandleId, C>,
        angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        unimplemented!()
    }

    fn working_level_exists<A, C, W>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        unimplemented!()
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        unimplemented!()
    }
}

#[derive(Default)]
struct TestStrategyParams;

impl StrategyParams for TestStrategyParams {
    type PointParam = StepPointParam;
    type RatioParam = StepRatioParam;

    fn get_point_param_value(&self, _name: Self::PointParam) -> ParamOutputValue {
        dec!(2)
    }

    fn get_ratio_param_value(
        &self,
        _name: Self::RatioParam,
        _volatility: CandleVolatility,
    ) -> ParamOutputValue {
        dec!(2)
    }
}

fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
    level_orders.is_empty()
}

fn level_expired_by_distance(
    level_price: WLPrice,
    _current_tick_price: UniversalTickPrice,
    _distance_from_level_for_its_deletion: ParamOutputValue,
) -> bool {
    level_price == dec!(1) || level_price == dec!(5)
}

fn level_expired_by_time(
    level_time: LevelTime,
    _current_tick_time: TickTime,
    _level_expiration: ParamOutputValue,
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
    _min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamOutputValue,
    _current_tick_price: UniversalTickPrice,
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
            .create_working_level(
                xid::new().to_string(),
                BacktestingWLProperties {
                    base: BasicWLProperties {
                        price: Decimal::from(i),
                        time: NaiveDate::from_ymd(2022, 1, i).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();

        if i == 4 {
            let order = store
                .create_order(
                    xid::new().to_string(),
                    StepOrderProperties {
                        working_level_id: level.id.clone(),
                        ..Default::default()
                    },
                )
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
            .create_working_level(
                xid::new().to_string(),
                BacktestingWLProperties {
                    base: BasicWLProperties {
                        price: Decimal::from(i),
                        time: NaiveDate::from_ymd(2022, 1, i).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();

        if i == 4 {
            let order = store
                .create_order(
                    xid::new().to_string(),
                    StepOrderProperties {
                        working_level_id: level.id.clone(),
                        ..Default::default()
                    },
                )
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
fn move_take_profits__realtime_tick__should_successfully_move_take_profits_of_active_levels_only() {
    let level_utils = LevelUtilsImpl::new();

    let mut store = InMemoryStepBacktestingStore::new();

    let take_profit = dec!(1.36800);
    let buy_wl_price = dec!(1.37000);
    let current_tick_price = UniversalTickPrice::Realtime(dec!(1.36799));
    let sell_wl_price = dec!(1.36598);

    for i in 0..8 {
        let level = store
            .create_working_level(
                xid::new().to_string(),
                BacktestingWLProperties {
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
                },
            )
            .unwrap();

        for _ in 0..3 {
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
                        working_level_id: level.id.clone(),
                    },
                )
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

#[test]
#[allow(non_snake_case)]
fn move_take_profits__historical_tick__should_successfully_move_take_profits_of_active_levels_only()
{
    let level_utils = LevelUtilsImpl::new();

    let mut store = InMemoryStepBacktestingStore::new();

    let take_profit = dec!(1.36800);
    let buy_wl_price = dec!(1.37000);

    let current_tick_price = UniversalTickPrice::Historical(HistoricalTickPrice {
        high: dec!(1.36801),
        low: dec!(1.36797),
        ..Default::default()
    });

    let sell_wl_price = dec!(1.36598);

    for i in 0..8 {
        let level = store
            .create_working_level(
                xid::new().to_string(),
                BacktestingWLProperties {
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
                },
            )
            .unwrap();

        for _ in 0..3 {
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
                        working_level_id: level.id.clone(),
                    },
                )
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

// update_tendency_and_get_instruction_to_create_new_working_level cases to test:
// - tendency is unknown, crossed angle is min, appropriate working level (should set tendency
//   to down and NOT return instruction to create new working level)
// - tendency is unknown, crossed angle is max, appropriate working level (should set tendency
//   to up and NOT return instruction to create new working level)
// - tendency is down, crossed angle is max, is not second level after bargaining tendency change,
//   level does NOT come out of bargaining corridor, appropriate working level (should update tendency
//   and return instruction to create new working level)
// - tendency is up, crossed angle is min, is not second level after bargaining tendency change,
//   level does NOT come out of bargaining corridor, the same working level already exists
//   (should update tendency and NOT return instruction to create new working level)
// - tendency is down, crossed angle is max, is not second level after bargaining tendency change,
//   level does NOT come out of bargaining corridor, inappropriate working level (should update
//   tendency and NOT return instruction to create new working level)
// - tendency is up, crossed angle is min, is not second level after bargaining tendency change,
//   level does NOT come out of bargaining corridor, working level is close to another one
//   (should update tendency and NOT return instruction to create new working level)
// - tendency is down, crossed angle is max, is not second level after bargaining tendency change,
//   level comes out of bargaining corridor, max level before bargaining corridor exists,
//   appropriate working level (should update tendency, set back max level to be max level before
//   bargaining corridor and NOT return instruction to create new working level)
// - tendency is up, crossed angle is min, is not second level after bargaining tendency change,
//   level comes out of bargaining corridor, min level before bargaining corridor exists,
//   appropriate working level (should update tendency, set back min level to be min level before
//   bargaining corridor and NOT return instruction to create new working level)
// - tendency is up, crossed angle is max, is second level after bargaining tendency change,
//   angle of second level after bargaining tendency change is None, appropriate working level
//   (should NOT update tendency, should set second level after bargaining tendency change
//   to be the crossed angle, should return instruction to create new working level)
// - tendency is down, crossed angle is min, is second level after bargaining tendency change,
//   angle of second level after bargaining tendency change exists, crossed angle equals to
//   angle of second level after bargaining tendency change, appropriate working level
//   (should NOT update tendency, should return instruction to create new working level)
// - tendency is up, crossed angle is max, is second level after bargaining tendency change,
//   angle of second level after bargaining tendency change exists, crossed angle doesn't equal to
//   angle of second level after bargaining tendency change, appropriate working level
//   (should NOT update tendency, should NOT return instruction to create new working level)

#[derive(Default)]
struct TestParams;

impl StrategyParams for TestParams {
    type PointParam = StepPointParam;
    type RatioParam = StepRatioParam;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamOutputValue {
        match name {
            StepPointParam::MinAmountOfCandlesInCorridorDefiningEdgeBargaining => dec!(5),
            _ => unimplemented!(),
        }
    }

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        _volatility: CandleVolatility,
    ) -> ParamOutputValue {
        match name {
            StepRatioParam::DistanceDefiningNearbyLevelsOfTheSameType => dec!(150),
            _ => unimplemented!(),
        }
    }
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_unknown_and_crossed_angle_is_min_and_appropriate_working_level__should_update_tendency_to_down_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig::default();
    let mut store = InMemoryStepBacktestingStore::new();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Down));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
        },
    };

    let current_candle = Item {
        id: String::from("2"),
        props: StepBacktestingCandleProperties::default(),
    };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Down);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_unknown_and_crossed_angle_is_max_and_appropriate_working_level__should_update_tendency_to_up_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig::default();
    let mut store = InMemoryStepBacktestingStore::new();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Up));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let crossed_angle = Item {
        id: String::from("1"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties::default(),
            },
        },
    };

    let current_candle = Item {
        id: String::from("2"),
        props: StepBacktestingCandleProperties::default(),
    };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Up);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_down_and_crossed_angle_is_max_and_is_not_second_level_after_bargaining_tendency_change_and_level_does_not_come_out_of_bargaining_corridor_and_appropriate_working_level__should_update_tendency_to_up_and_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Down,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: true,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Up));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Up);
    assert!(!config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);

    assert_eq!(statistics.number_of_tendency_changes, 1);

    assert_eq!(
        store.get_tendency_change_angle().unwrap().unwrap(),
        crossed_angle
    );

    assert!(store
        .get_angle_of_second_level_after_bargaining_tendency_change()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_up_and_crossed_angle_is_min_and_is_not_second_level_after_bargaining_tendency_change_and_level_does_not_come_out_of_bargaining_corridor_and_same_working_level_exists__should_update_tendency_to_down_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Up,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: true,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(true)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Down));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Down);
    assert!(!config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);

    assert_eq!(statistics.number_of_tendency_changes, 1);

    assert_eq!(
        store.get_tendency_change_angle().unwrap().unwrap(),
        crossed_angle
    );

    assert!(store
        .get_angle_of_second_level_after_bargaining_tendency_change()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_down_and_crossed_angle_is_max_and_is_not_second_level_after_bargaining_tendency_change_and_level_does_not_come_out_of_bargaining_corridor_and_inappropriate_working_level__should_update_tendency_to_up_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Down,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: true,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(false)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Up));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Up);
    assert!(!config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);

    assert_eq!(statistics.number_of_tendency_changes, 1);

    assert_eq!(
        store.get_tendency_change_angle().unwrap().unwrap(),
        crossed_angle
    );

    assert!(store
        .get_angle_of_second_level_after_bargaining_tendency_change()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_up_and_crossed_angle_is_min_and_is_not_second_level_after_bargaining_tendency_change_and_level_does_not_come_out_of_bargaining_corridor_and_working_level_is_close_to_another_one__should_update_tendency_to_down_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Up,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: true,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(true)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Down));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Down);
    assert!(!config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);

    assert_eq!(statistics.number_of_tendency_changes, 1);

    assert_eq!(
        store.get_tendency_change_angle().unwrap().unwrap(),
        crossed_angle
    );

    assert!(store
        .get_angle_of_second_level_after_bargaining_tendency_change()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_down_and_crossed_angle_is_max_and_is_not_second_level_after_bargaining_tendency_change_and_level_comes_out_of_bargaining_corridor_and_max_angle_before_bargaining_corridor_exists__should_update_tendency_to_up_and_set_back_max_angle_to_be_max_angle_before_bargaining_corridor_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Down,
        tendency_changed_on_crossing_bargaining_corridor: false,
        second_level_after_bargaining_tendency_change_is_created: true,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    let max_angle_before_bargaining_corridor_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let max_angle_before_bargaining_corridor = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            max_angle_before_bargaining_corridor_candle.id,
        )
        .unwrap();

    store
        .update_max_angle_before_bargaining_corridor(
            max_angle_before_bargaining_corridor.id.clone(),
        )
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(true)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Up));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Up);
    assert!(config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);

    assert_eq!(statistics.number_of_tendency_changes, 1);

    assert_eq!(
        store.get_tendency_change_angle().unwrap().unwrap(),
        crossed_angle
    );

    assert_eq!(
        store.get_max_angle().unwrap().unwrap(),
        max_angle_before_bargaining_corridor
    );

    assert!(store
        .get_angle_of_second_level_after_bargaining_tendency_change()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_up_and_crossed_angle_is_min_and_is_not_second_level_after_bargaining_tendency_change_and_level_comes_out_of_bargaining_corridor_and_min_angle_before_bargaining_corridor_exists__should_update_tendency_to_down_and_set_back_min_angle_to_be_min_angle_before_bargaining_corridor_and_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Up,
        tendency_changed_on_crossing_bargaining_corridor: false,
        second_level_after_bargaining_tendency_change_is_created: true,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    let min_angle_before_bargaining_corridor_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let min_angle_before_bargaining_corridor = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            min_angle_before_bargaining_corridor_candle.id,
        )
        .unwrap();

    store
        .update_min_angle_before_bargaining_corridor(
            min_angle_before_bargaining_corridor.id.clone(),
        )
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        false
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(true)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let number_of_calls_to_add_entity_to_chart_traces = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {
            assert_eq!(entity, ChartTraceEntity::Tendency(Tendency::Down));
            *number_of_calls_to_add_entity_to_chart_traces.borrow_mut() += 1;
        };

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Down);
    assert!(config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(*number_of_calls_to_add_entity_to_chart_traces.borrow(), 1);

    assert_eq!(statistics.number_of_tendency_changes, 1);

    assert_eq!(
        store.get_tendency_change_angle().unwrap().unwrap(),
        crossed_angle
    );

    assert_eq!(
        store.get_min_angle().unwrap().unwrap(),
        min_angle_before_bargaining_corridor
    );

    assert!(store
        .get_angle_of_second_level_after_bargaining_tendency_change()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_up_and_crossed_angle_is_max_and_is_second_level_after_bargaining_tendency_change_and_angle_of_second_level_after_bargaining_tendency_change_is_none_and_appropriate_working_level__should_not_update_tendency_and_should_set_second_level_after_bargaining_tendency_change_to_be_crossed_angle_and_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Up,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: false,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        true
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let add_entity_to_chart_traces =
        |_entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {};

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Up);
    assert!(config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(statistics.number_of_tendency_changes, 0);

    assert!(store.get_tendency_change_angle().unwrap().is_none());

    assert_eq!(
        store
            .get_angle_of_second_level_after_bargaining_tendency_change()
            .unwrap()
            .unwrap(),
        crossed_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_down_and_crossed_angle_is_min_and_is_second_level_after_bargaining_tendency_change_and_angle_of_second_level_after_bargaining_tendency_change_is_none_and_appropriate_working_level__should_not_update_tendency_and_should_set_second_level_after_bargaining_tendency_change_to_be_crossed_angle_and_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Down,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: false,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        true
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let add_entity_to_chart_traces =
        |_entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {};

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Down);
    assert!(config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(statistics.number_of_tendency_changes, 0);

    assert!(store.get_tendency_change_angle().unwrap().is_none());

    assert_eq!(
        store
            .get_angle_of_second_level_after_bargaining_tendency_change()
            .unwrap()
            .unwrap(),
        crossed_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_up_and_crossed_angle_is_max_and_is_second_level_after_bargaining_tendency_change_and_angle_of_second_level_after_bargaining_tendency_change_exists_and_crossed_angle_equals_to_angle_of_second_level_and_appropriate_working_level__should_not_update_tendency_and_should_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Up,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: false,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            crossed_angle.id.clone(),
        ))
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        true
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let add_entity_to_chart_traces =
        |_entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {};

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Up);
    assert!(config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(statistics.number_of_tendency_changes, 0);

    assert!(store.get_tendency_change_angle().unwrap().is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_tendency_and_get_instruction_to_create_new_working_level__tendency_is_down_and_crossed_angle_is_min_and_is_second_level_after_bargaining_tendency_change_and_angle_of_second_level_after_bargaining_tendency_change_exists_and_crossed_angle_does_not_equal_to_angle_of_second_level_and_appropriate_working_level__should_not_update_tendency_and_should_not_return_instruction_to_create_new_working_level(
) {
    let mut config = StepConfig {
        tendency: Tendency::Down,
        tendency_changed_on_crossing_bargaining_corridor: true,
        second_level_after_bargaining_tendency_change_is_created: false,
        ..Default::default()
    };

    let mut store = InMemoryStepBacktestingStore::new();

    let crossed_angle_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();
    let crossed_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            crossed_angle_candle.id,
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    let angle_of_second_level_after_bargaining_tendency_change = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                ..Default::default()
            },
            angle_of_second_level_after_bargaining_tendency_change_candle.id,
        )
        .unwrap();

    store
        .update_angle_of_second_level_after_bargaining_tendency_change(Some(
            angle_of_second_level_after_bargaining_tendency_change.id,
        ))
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties::default(),
        )
        .unwrap();

    fn is_second_level_after_bargaining_tendency_change(
        _crossed_angle: &str,
        _tendency_change_angle: Option<&str>,
        _last_tendency_changed_on_crossing_bargaining_corridor: bool,
        _second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        true
    }

    fn level_comes_out_of_bargaining_corridor<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _general_corridor: &[Item<CandleId, C>],
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        Ok(false)
    }

    fn appropriate_working_level<A, C>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _current_candle: &Item<CandleId, C>,
        _angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        _params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
    {
        Ok(true)
    }

    fn working_level_exists<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>,
    {
        Ok(false)
    }

    fn working_level_is_close_to_another_one<A, C, W>(
        _crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        _working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        _distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug,
    {
        Ok(false)
    }

    let mut statistics = StepBacktestingStatistics::default();

    let add_entity_to_chart_traces =
        |_entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_chart_index: ChartIndex| {};

    let mut chart_traces = StepBacktestingChartTraces::new(10);

    let statistics_charts_notifier: StatisticsChartsNotifier<FakeBacktestingNotificationQueue, _> =
        StatisticsChartsNotifier::Backtesting {
            statistics: &mut statistics,
            add_entity_to_chart_traces: &add_entity_to_chart_traces,
            chart_traces: &mut chart_traces,
            current_candle_chart_index: 5,
            crossed_angle_candle_chart_index: 7,
        };

    let params = TestParams::default();

    env::set_var("MODE", "debug");

    assert!(
        !LevelUtilsImpl::update_tendency_and_get_instruction_to_create_new_working_level(
            &mut config,
            &mut store,
            UpdateTendencyAndCreateWorkingLevelUtils::new(
                &is_second_level_after_bargaining_tendency_change,
                &level_comes_out_of_bargaining_corridor,
                &appropriate_working_level,
                &working_level_exists,
                &working_level_is_close_to_another_one,
            ),
            statistics_charts_notifier,
            &crossed_angle,
            &current_candle,
            &params,
        )
        .unwrap()
    );

    assert_eq!(config.tendency, Tendency::Down);
    assert!(config.tendency_changed_on_crossing_bargaining_corridor);
    assert!(!config.second_level_after_bargaining_tendency_change_is_created);

    assert_eq!(statistics.number_of_tendency_changes, 0);

    assert!(store.get_tendency_change_angle().unwrap().is_none());
}
