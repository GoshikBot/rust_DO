use super::*;
use crate::step::utils::entities::candle::{
    StepBacktestingCandleProperties, StepCandleProperties,
};
use crate::step::utils::entities::order::StepOrderProperties;
use crate::step::utils::entities::working_levels::BacktestingWLProperties;
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use base::entities::candle::CandleVolatility;
use base::entities::CandlePrices;
use base::stores::candle_store::BasicCandleStore;
use rust_decimal_macros::dec;

#[derive(Default)]
struct TestParams;

impl StrategyParams for TestParams {
    type PointParam = StepPointParam;
    type RatioParam = StepRatioParam;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamOutputValue {
        match name {
            StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct => dec!(20),
            StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel => dec!(3),
            _ => unreachable!()
        }
    }

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        _volatility: CandleVolatility,
    ) -> ParamOutputValue {
        match name {
            StepRatioParam::RangeOfBigCorridorNearLevel => dec!(200),
            StepRatioParam::DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel => {
                dec!(30)
            }
            _ => unreachable!(),
        }
    }
}

// Corridor configs to test:
// - small corridor:
// 1.  corridor is empty && candle can be corridor leader
// 2.  corridor is empty && candle can't be corridor leader
// 3.  corridor is not empty && candle is in corridor
// 4.  corridor is not empty && candle is NOT in corridor && candle is less than distance
//     && enough candles in corridor
// 5.  corridor is NOT empty && candle is NOT in corridor && candle is less than distance
//     && not enough candles in corridor yet && new corridor is empty && candle can be corridor leader
// 6.  corridor is NOT empty && candle is NOT in corridor && candle is less than distance
//     && not enough candles in corridor yet && new corridor is empty && candle can't be corridor leader
// 7.  corridor is NOT empty && candle is NOT in corridor && candle is less than distance
//     && not enough candles in corridor yet && new corridor is not empty
// 8.  corridor is NOT empty && candle is NOT in corridor && candle is greater than distance
//     && new corridor is NOT empty
// 9.  corridor is NOT empty && candle is NOT in corridor && candle is greater than distance
//     && new corridor is empty
//
// - big corridor:
// 1.  buy level && green candle && candle is in the range of the corridor
// 2.  buy level && neutral candle && candle is in the range of the corridor
// 3.  buy level && red candle && candle is in the range of the corridor
// 4.  buy level && green candle && candle is NOT in the range of the corridor
// 5.  buy level && neutral candle && candle is NOT in the range of the corridor
// 6.  buy level && red candle && candle is NOT in the range of the corridor
// 7.  sell level && green candle && candle is in the range of the corridor
// 8.  sell level && neutral candle && candle is in the range of the corridor
// 9.  sell level && red candle && candle is in the range of the corridor
// 10. sell level && green candle && candle is NOT in the range of the corridor
// 11. sell level && neutral candle && candle is NOT in the range of the corridor
// 12. sell level && red candle && candle is NOT in the range of the corridor

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_empty_and_candle_can_be_corridor_leader_and_buy_level_and_green_candle_and_candle_is_in_the_range_of_big_corridor__should_add_candle_to_small_and_big_corridors(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Green,
                        prices: CandlePrices {
                            close: dec!(1.38199),
                            low: dec!(1.38029),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;
    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.len() == 1 && small_corridor.contains(&current_candle));

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.len() == 1 && big_corridor.contains(&current_candle));
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_empty_and_candle_cannot_be_corridor_leader_and_buy_level_and_neutral_candle_and_candle_is_in_the_range_of_big_corridor__should_add_candle_to_big_corridor_and_not_small(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Neutral,
                        prices: CandlePrices {
                            close: dec!(1.38199),
                            low: dec!(1.38029),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;
    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.is_empty());

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.len() == 1 && big_corridor.contains(&current_candle));
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_in_corridor_and_buy_level_and_red_candle_and_candle_is_in_the_range_of_big_corridor__should_add_candle_to_small_and_big_corridors(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let corridor_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();
    store
        .add_candle_to_working_level_corridor(
            &working_level.id,
            corridor_candle.id,
            CorridorType::Small,
        )
        .unwrap();

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Red,
                        prices: CandlePrices {
                            open: dec!(1.38199),
                            low: dec!(1.38031),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| true;
    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.len() == 2 && small_corridor.contains(&current_candle));

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.len() == 1 && big_corridor.contains(&current_candle));
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_not_in_corridor_and_candle_is_in_range_of_small_corridor_and_enough_candles_in_small_corridor_and_buy_level_and_green_candle_and_candle_is_not_in_the_range_of_big_corridor__should_not_add_candle_neither_to_small_nor_to_big_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..3 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id.clone(),
                CorridorType::Small,
            )
            .unwrap();

        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Big,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Green,
                        prices: CandlePrices {
                            close: dec!(1.38201),
                            low: dec!(1.38029),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;
    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.len() == 3 && !small_corridor.contains(&current_candle));

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.is_empty());
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_not_in_corridor_and_candle_is_in_range_of_small_corridor_and_not_enough_candles_in_small_corridor_and_new_corridor_is_empty_and_candle_can_be_corridor_leader_and_buy_level_and_neutral_candle_and_candle_is_not_in_the_range_of_big_corridor__should_add_candle_to_small_corridor_and_not_big(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..2 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id.clone(),
                CorridorType::Small,
            )
            .unwrap();

        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Big,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Neutral,
                        prices: CandlePrices {
                            close: dec!(1.38201),
                            low: dec!(1.38029),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;
    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.len() == 1 && small_corridor.contains(&current_candle));

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.is_empty());
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_not_in_corridor_and_candle_is_in_range_of_small_corridor_and_not_enough_candles_in_small_corridor_and_new_corridor_is_empty_and_candle_cannot_be_corridor_leader_and_buy_level_and_red_candle_and_candle_is_not_in_the_range_of_big_corridor__should_not_add_candle_neither_to_small_nor_to_big_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..2 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id.clone(),
                CorridorType::Small,
            )
            .unwrap();

        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Big,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Red,
                        prices: CandlePrices {
                            open: dec!(1.38201),
                            low: dec!(1.38029),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;
    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.is_empty());

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.is_empty());
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_not_in_corridor_and_candle_is_in_range_of_small_corridor_and_not_enough_candles_in_small_corridor_and_new_corridor_is_not_empty_and_sell_level_and_green_candle_and_candle_is_in_the_range_of_big_corridor__should_set_new_small_corridor_and_add_candle_to_big_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..2 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Small,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Green,
                        prices: CandlePrices {
                            open: dec!(1.37801),
                            high: dec!(1.37971),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let mut new_corridor = Vec::new();

    for _ in 0..3 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        new_corridor.push(candle);
    }

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| Some(new_corridor.clone());

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert_eq!(small_corridor, new_corridor);

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.len() == 1 && big_corridor.contains(&current_candle));
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_not_in_corridor_and_candle_is_not_in_range_of_small_corridor_and_not_enough_candles_in_small_corridor_and_new_corridor_is_not_empty_and_sell_level_and_neutral_candle_and_candle_is_in_the_range_of_big_corridor__should_set_new_small_corridor_and_add_candle_to_big_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..2 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Small,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Neutral,
                        prices: CandlePrices {
                            open: dec!(1.37801),
                            high: dec!(1.37969),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let mut new_corridor = Vec::new();

    for _ in 0..3 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        new_corridor.push(candle);
    }

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| Some(new_corridor.clone());

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert_eq!(small_corridor, new_corridor);

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.len() == 1 && big_corridor.contains(&current_candle));
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__small_corridor_is_not_empty_and_candle_is_not_in_corridor_and_candle_is_not_in_range_of_small_corridor_and_not_enough_candles_in_small_corridor_and_new_corridor_is_empty_and_sell_level_and_red_candle_and_candle_is_in_the_range_of_big_corridor__should_clear_small_corridor_and_add_candle_to_big_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..2 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Small,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Red,
                        prices: CandlePrices {
                            close: dec!(1.37801),
                            high: dec!(1.37969),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let small_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Small)
        .unwrap();

    assert!(small_corridor.is_empty());

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.len() == 1 && big_corridor.contains(&current_candle));
}

#[test]
#[allow(non_snake_case)]
fn update_corridors_near_working_levels__sell_level_and_red_candle_and_candle_is_not_in_the_range_of_big_corridor__should_clear_big_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::new();
    let working_level = store
        .create_working_level(
            xid::new().to_string(),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    for _ in 0..3 {
        let corridor_candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();

        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id.clone(),
                CorridorType::Big,
            )
            .unwrap();
    }

    let current_candle = store
        .create_candle(
            xid::new().to_string(),
            StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Red,
                        prices: CandlePrices {
                            close: dec!(1.37799),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

    let params = TestParams::default();

    CorridorsImpl::update_corridors_near_working_levels(
        &mut store,
        &current_candle,
        UpdateCorridorsNearWorkingLevelsUtils::new(
            UpdateGeneralCorridorUtils::new(
                &candle_can_be_corridor_leader,
                &candle_is_in_corridor,
                &crop_corridor_to_the_closest_leader,
            ),
            &level_has_no_active_orders,
        ),
        &params,
    )
        .unwrap();

    let big_corridor = store
        .get_candles_of_working_level_corridor(&working_level.id, CorridorType::Big)
        .unwrap();

    assert!(big_corridor.is_empty());
}

// update_general_corridor cases to test:
// - corridor is empty && candle can be corridor leader (should add corridor leader)
// - corridor is empty && candle can't be corridor leader (should leave corridor empty)
// - corridor is NOT empty && candle is in corridor (should add candle to corridor)
// - corridor is NOT empty && candle is NOT in corridor && new cropped corridor is NOT empty
//   (should replace corridor with new cropped corridor)
// - corridor is NOT empty && candle is NOT in corridor && new cropped corridor is empty
//   && candle can be corridor leader (should add corridor leader)
// - corridor is NOT empty && candle is NOT in corridor && new cropped corridor is empty
//   && candle can't be corridor leader (should clear corridor)

#[test]
#[allow(non_snake_case)]
fn update_general_corridor__corridor_is_empty_and_candle_can_be_corridor_leader__should_set_new_corridor_leader(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let current_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    store
        .update_current_candle(current_candle.id.clone())
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;

    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    CorridorsImpl::update_general_corridor(
        &current_candle,
        &mut store,
        UpdateGeneralCorridorUtils::new(
            &candle_can_be_corridor_leader,
            &candle_is_in_corridor,
            &crop_corridor_to_the_closest_leader,
        ),
        dec!(20),
    )
        .unwrap();

    let general_corridor = store.get_candles_of_general_corridor().unwrap();

    assert_eq!(general_corridor.len(), 1);
    assert_eq!(general_corridor[0], current_candle);
}

#[test]
#[allow(non_snake_case)]
fn update_general_corridor__corridor_is_empty_and_candle_cannot_be_corridor_leader__should_leave_corridor_empty(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let current_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    store
        .update_current_candle(current_candle.id.clone())
        .unwrap();

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;

    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    CorridorsImpl::update_general_corridor(
        &current_candle,
        &mut store,
        UpdateGeneralCorridorUtils::new(
            &candle_can_be_corridor_leader,
            &candle_is_in_corridor,
            &crop_corridor_to_the_closest_leader,
        ),
        dec!(20),
    )
        .unwrap();

    assert!(store.get_candles_of_general_corridor().unwrap().is_empty());
}

#[test]
#[allow(non_snake_case)]
fn update_general_corridor__corridor_is_not_empty_and_candle_is_in_corridor__should_add_candle_to_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let current_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    store
        .update_current_candle(current_candle.id.clone())
        .unwrap();

    for _ in 0..3 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store.add_candle_to_general_corridor(candle.id).unwrap();
    }

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;

    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| true;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    CorridorsImpl::update_general_corridor(
        &current_candle,
        &mut store,
        UpdateGeneralCorridorUtils::new(
            &candle_can_be_corridor_leader,
            &candle_is_in_corridor,
            &crop_corridor_to_the_closest_leader,
        ),
        dec!(20),
    )
        .unwrap();

    let general_corridor = store.get_candles_of_general_corridor().unwrap();

    assert_eq!(general_corridor.len(), 4);
    assert_eq!(general_corridor[3], current_candle);
}

#[test]
#[allow(non_snake_case)]
fn update_general_corridor__corridor_is_not_empty_and_candle_is_not_in_corridor_and_new_cropped_corridor_is_not_empty__should_replace_corridor_with_new_cropped_one(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let current_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    store
        .update_current_candle(current_candle.id.clone())
        .unwrap();

    for _ in 0..3 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store.add_candle_to_general_corridor(candle.id).unwrap();
    }

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;

    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let mut new_cropped_corridor = Vec::new();

    for _ in 0..2 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        new_cropped_corridor.push(candle);
    }

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| Some(new_cropped_corridor.clone());

    CorridorsImpl::update_general_corridor(
        &current_candle,
        &mut store,
        UpdateGeneralCorridorUtils::new(
            &candle_can_be_corridor_leader,
            &candle_is_in_corridor,
            &crop_corridor_to_the_closest_leader,
        ),
        dec!(20),
    )
        .unwrap();

    assert_eq!(
        store.get_candles_of_general_corridor().unwrap(),
        new_cropped_corridor
    );
}

#[test]
#[allow(non_snake_case)]
fn update_general_corridor__corridor_is_not_empty_and_candle_is_not_in_corridor_and_new_cropped_corridor_is_empty_and_candle_can_be_corridor_leader__should_set_new_corridor_leader(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let current_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    store
        .update_current_candle(current_candle.id.clone())
        .unwrap();

    for _ in 0..3 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store.add_candle_to_general_corridor(candle.id).unwrap();
    }

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;

    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    CorridorsImpl::update_general_corridor(
        &current_candle,
        &mut store,
        UpdateGeneralCorridorUtils::new(
            &candle_can_be_corridor_leader,
            &candle_is_in_corridor,
            &crop_corridor_to_the_closest_leader,
        ),
        dec!(20),
    )
        .unwrap();

    let general_corridor = store.get_candles_of_general_corridor().unwrap();

    assert_eq!(general_corridor.len(), 1);
    assert_eq!(general_corridor[0], current_candle);
}

#[test]
#[allow(non_snake_case)]
fn update_general_corridor__corridor_is_not_empty_and_candle_is_not_in_corridor_and_new_cropped_corridor_is_empty_and_candle_can_not_be_corridor_leader__should_clear_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let current_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    store
        .update_current_candle(current_candle.id.clone())
        .unwrap();

    for _ in 0..3 {
        let candle = store
            .create_candle(xid::new().to_string(), Default::default())
            .unwrap();
        store.add_candle_to_general_corridor(candle.id).unwrap();
    }

    let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;

    let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                 _: &StepBacktestingCandleProperties,
                                 _: ParamOutputValue| false;

    let crop_corridor_to_the_closest_leader =
        |_: &[Item<CandleId, StepBacktestingCandleProperties>],
         _: &Item<CandleId, StepBacktestingCandleProperties>,
         _: ParamOutputValue,
         _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
         _: &dyn Fn(
             &StepBacktestingCandleProperties,
             &StepBacktestingCandleProperties,
             ParamOutputValue,
         ) -> bool| None;

    CorridorsImpl::update_general_corridor(
        &current_candle,
        &mut store,
        UpdateGeneralCorridorUtils::new(
            &candle_can_be_corridor_leader,
            &candle_is_in_corridor,
            &crop_corridor_to_the_closest_leader,
        ),
        dec!(20),
    )
        .unwrap();

    assert!(store.get_candles_of_general_corridor().unwrap().is_empty());
}
