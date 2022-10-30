use super::*;
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use base::entities::candle::BasicCandleProperties;
use base::entities::{CandlePrices, CandleType};
use base::stores::candle_store::BasicCandleStore;
use rust_decimal_macros::dec;

#[test]
#[allow(non_snake_case)]
fn get_diff_between_current_and_previous_candles__current_candle_is_greater_than_previous__should_return_greater(
) {
    let current_candle_props = StepCandleProperties {
        leading_price: dec!(1.38000),
        ..Default::default()
    };
    let previous_candle_props = StepCandleProperties {
        leading_price: dec!(1.37950),
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_diff_between_current_and_previous_candles(
            &current_candle_props,
            &previous_candle_props
        ),
        Diff::Greater
    );
}

#[test]
#[allow(non_snake_case)]
fn get_diff_between_current_and_previous_candles__current_candle_is_less_than_previous__should_return_greater(
) {
    let current_candle_props = StepCandleProperties {
        leading_price: dec!(1.38000),
        ..Default::default()
    };
    let previous_candle_props = StepCandleProperties {
        leading_price: dec!(1.38100),
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_diff_between_current_and_previous_candles(
            &current_candle_props,
            &previous_candle_props
        ),
        Diff::Less
    );
}

#[test]
#[allow(non_snake_case)]
fn get_diff_between_current_and_previous_candles__current_candle_is_equal_to_previous_and_leading_price_is_equal_to_high__should_return_greater(
) {
    let current_candle_props = StepCandleProperties {
        leading_price: dec!(1.38000),
        base: BasicCandleProperties {
            prices: CandlePrices {
                high: dec!(1.38000),
                ..Default::default()
            },
            ..Default::default()
        },
    };
    let previous_candle_props = StepCandleProperties {
        leading_price: dec!(1.38000),
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_diff_between_current_and_previous_candles(
            &current_candle_props,
            &previous_candle_props
        ),
        Diff::Greater
    );
}

#[test]
#[allow(non_snake_case)]
fn get_diff_between_current_and_previous_candles__current_candle_is_equal_to_previous_and_leading_price_is_equal_to_low__should_return_less(
) {
    let current_candle_props = StepCandleProperties {
        leading_price: dec!(1.38000),
        base: BasicCandleProperties {
            prices: CandlePrices {
                low: dec!(1.38000),
                ..Default::default()
            },
            ..Default::default()
        },
    };
    let previous_candle_props = StepCandleProperties {
        leading_price: dec!(1.38000),
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_diff_between_current_and_previous_candles(
            &current_candle_props,
            &previous_candle_props
        ),
        Diff::Less
    );
}

// get_new_angle configs to test
// - no new angle diffs
// - new max angle with high leading price, neither max nor min angles exist
// - new max angle with low leading price, neither max nor min angles exist
// - new min angle with low leading price, neither max nor min angles exist
// - new min angle with high leading price, neither max nor min angles exist
//
// - new max angle, max angle exists, no min angle, new angle crossed previous
// - new max angle, max angle exists, no min angle, new angle has NOT crossed previous
// - new max angle, min angle exists, no max angle, appropriate distance between
//   current min and new max angle
// - new max angle, min angle exists, no max angle, inappropriate distance between
//   current min and new max angle
// - new max angle, both min and max angles exist, new angle crossed previous
// - new max angle, both min and max angles exist, new angle has NOT crossed previous,
//   inappropriate distance between current min and new max angle
// - new max angle, both min and max angles exist, new angle has NOT crossed previous,
//   appropriate distance between current min and new max angle, appropriate distance
//   between current min and max angles for new inner angle to appear
// - new max angle, both min and max angles exist, new angle has NOT crossed previous,
//   appropriate distance between current min and new max angle, inappropriate distance
//   between current min and max angles for new inner angle to appear
//
// - new min angle, min angle exists, no max angle, new angle crossed previous
// - new min angle, min angle exists, no max angle, new angle has NOT crossed previous
// - new min angle, max angle exists, no min angle, appropriate distance between
//   current max and new min angle
// - new min angle, max angle exists, no min angle, inappropriate distance between
//   current max and new min angle
// - new min angle, both min and max angles exist, new angle crossed previous
// - new min angle, both min and max angles exist, new angle has NOT crossed previous,
//   inappropriate distance between current max and new min angle
// - new min angle, both min and max angles exist, new angle has NOT crossed previous,
//   appropriate distance between current max and new min angle, appropriate distance
//   between current min and max angles for new inner angle to appear
// - new min angle, both min and max angles exist, new angle has NOT crossed previous,
//   appropriate distance between current max and new min angle, inappropriate distance
//   between current min and max angles for new inner angle to appear

#[test]
#[allow(non_snake_case)]
fn get_new_angle__no_new_angle_diffs__should_return_none() {
    let previous_candle = Item {
        id: String::from("1"),
        props: StepCandleProperties::default(),
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Greater,
    };

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear = dec!(1);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_with_high_leading_price_and_neither_max_nor_min_angles_exist__should_return_new_max_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Max,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_with_low_leading_price_and_neither_max_nor_min_angles_exist__should_return_none(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_with_low_leading_price_and_neither_max_nor_min_angles_exist__should_return_new_min_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Min,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_with_high_leading_price_and_neither_max_nor_min_angles_exist__should_return_none(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_max_angle_exists_and_no_min_angle_and_new_angle_crossed_previous__should_return_new_max_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Max,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_max_angle_exists_and_no_min_angle_and_new_angle_has_not_crossed_previous__should_return_none(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_min_angle_exists_and_no_max_angle_and_appropriate_distance_between_current_min_and_new_max_angle__should_return_new_max_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &None,
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Max,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_min_angle_exists_and_no_max_angle_and_inappropriate_distance_between_current_min_and_new_max_angle__should_return_none(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &None,
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_001);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_both_min_and_max_angles_exist_and_new_angle_crossed_previous__should_return_new_max_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.37900),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37900),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Max,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_both_min_and_max_angles_exist_and_new_angle_has_not_crossed_previous_and_inappropriate_distance_between_current_min_and_new_max_angles__should_return_none(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.38100),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.38100),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_001);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_both_min_and_max_angles_exist_and_new_angle_has_not_crossed_previous_and_appropriate_distance_between_current_min_and_new_max_angles_and_appropriate_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear__should_return_new_max_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.38100),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.38100),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_100);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Max,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_max_angle_and_both_min_and_max_angles_exist_and_new_angle_has_not_crossed_previous_and_appropriate_distance_between_current_min_and_new_max_angles_and_inappropriate_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear__should_return_new_virtual_max_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Less,
        previous: Diff::Greater,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.38100),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.38100),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_101);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Max,
            state: AngleState::Virtual,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_min_angle_exists_and_no_max_angle_and_new_angle_crossed_previous__should_return_new_min_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &None,
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.38100),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.38100),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Min,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_min_angle_exists_and_no_max_angle_and_new_angle_has_not_crossed_previous__should_return_none(
) {
    let previous_candle = Item {
        id: String::from("1"),
        props: StepCandleProperties {
            base: BasicCandleProperties {
                prices: CandlePrices {
                    low: dec!(1.38200),
                    ..Default::default()
                },
                ..Default::default()
            },
            leading_price: dec!(1.38200),
        },
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &None,
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.38100),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.38100),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_max_angle_exists_and_no_min_angle_and_appropriate_distance_between_current_max_and_new_min_angle__should_return_new_min_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Min,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_max_angle_exists_and_no_min_angle_and_inappropriate_distance_between_current_max_and_new_min_angle__should_return_none(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &None,
    };

    let min_distance_between_new_and_current_angles = dec!(1_001);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_both_min_and_max_angles_exist_and_new_angle_crossed_previous__should_return_new_min_angle(
) {
    let previous_candle = Item {
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
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.38100),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.38100),
                    },
                },
            },
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(1_000_000);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Min,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_both_min_and_max_angles_exist_and_new_angle_has_not_crossed_previous_and_inappropriate_distance_between_current_max_and_new_min_angles__should_return_none(
) {
    let previous_candle = Item {
        id: String::from("1"),
        props: StepCandleProperties {
            base: BasicCandleProperties {
                prices: CandlePrices {
                    low: dec!(1.38500),
                    ..Default::default()
                },
                ..Default::default()
            },
            leading_price: dec!(1.38500),
        },
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
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
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(501);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000_000);

    assert!(AngleUtilsImpl::get_new_angle(
        &previous_candle,
        diffs,
        angles,
        min_distance_between_new_and_current_angles,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
    )
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_both_min_and_max_angles_exist_and_new_angle_has_not_crossed_previous_and_appropriate_distance_between_current_max_and_new_min_angles_and_appropriate_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear__should_return_new_min_angle(
) {
    let previous_candle = Item {
        id: String::from("1"),
        props: StepCandleProperties {
            base: BasicCandleProperties {
                prices: CandlePrices {
                    low: dec!(1.38500),
                    ..Default::default()
                },
                ..Default::default()
            },
            leading_price: dec!(1.38500),
        },
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
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
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(500);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_000);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Min,
            state: AngleState::Real,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_new_angle__new_min_angle_and_both_min_and_max_angles_exist_and_new_angle_has_not_crossed_previous_and_appropriate_distance_between_current_max_and_new_min_angles_and_inappropriate_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear__should_return_new_virtual_min_angle(
) {
    let previous_candle = Item {
        id: String::from("1"),
        props: StepCandleProperties {
            base: BasicCandleProperties {
                prices: CandlePrices {
                    low: dec!(1.38500),
                    ..Default::default()
                },
                ..Default::default()
            },
            leading_price: dec!(1.38500),
        },
    };

    let diffs = ExistingDiffs {
        current: Diff::Greater,
        previous: Diff::Less,
    };

    let angles = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("3"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
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
        }),
    };

    let min_distance_between_new_and_current_angles = dec!(500);
    let min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear =
        dec!(1_001);

    let expected_new_angle = FullAngleProperties {
        base: BasicAngleProperties {
            r#type: Level::Min,
            state: AngleState::Virtual,
        },
        candle: previous_candle.clone(),
    };

    assert_eq!(
        AngleUtilsImpl::get_new_angle(
            &previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
        )
            .unwrap(),
        expected_new_angle
    );
}

// update_angles configs to test:
// - new virtual min angle
// - new virtual max angle
//
// - new real min angle, new angle is NOT in the bargaining corridor
// - new real min angle, new angle is in the bargaining corridor, previous min angle doesn't exist,
// - new real min angle, new angle is in the bargaining corridor, previous min angle exists,
//   previous min angle is in the bargaining corridor
// - new real min angle, new angle is in the bargaining corridor, previous min angle exists,
//   previous min angle is NOT in the bargaining corridor
//
// - new real max angle, new angle is NOT in the bargaining corridor
// - new real max angle, new angle is in the bargaining corridor, previous max angle doesn't exist,
// - new real max angle, new angle is in the bargaining corridor, previous max angle exists,
//   previous max angle is in the bargaining corridor
// - new real max angle, new angle is in the bargaining corridor, previous max angle exists,
//   previous max angle is NOT in the bargaining corridor
#[test]
#[allow(non_snake_case)]
fn update_angles__new_virtual_min_angle__should_update_virtual_min_angle() {
    let mut store = InMemoryStepBacktestingStore::default();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Virtual,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

    assert_eq!(store.get_virtual_min_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_virtual_max_angle().unwrap().is_none());
    assert!(store.get_min_angle().unwrap().is_none());
    assert!(store.get_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_virtual_max_angle__should_update_virtual_max_angle() {
    let mut store = InMemoryStepBacktestingStore::default();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Virtual,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

    assert_eq!(store.get_virtual_max_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_min_angle().unwrap().is_none());
    assert!(store.get_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_min_angle_and_new_angle_is_not_in_bargaining_corridor__should_update_min_angle(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

    assert_eq!(store.get_min_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_max_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_min_angle_and_new_angle_is_in_bargaining_corridor_and_previous_min_angle_does_not_exist__should_update_min_angle(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    let general_corridor = vec![new_angle.props.candle.clone()];

    AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

    assert_eq!(store.get_min_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_max_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_min_angle_and_new_angle_is_in_bargaining_corridor_and_previous_min_angle_exists_and_previous_min_angle_is_in_bargaining_corridor__should_update_min_angle(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let previous_angle_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    let previous_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            previous_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_min_angle(previous_angle.id).unwrap();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    let general_corridor = vec![new_angle.props.candle.clone(), previous_angle_candle];

    AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

    assert_eq!(store.get_min_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_max_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_min_angle_and_new_angle_is_in_bargaining_corridor_and_previous_min_angle_exists_and_previous_min_angle_is_not_in_bargaining_corridor__should_update_real_min_angle_and_min_angle_before_bargaining_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let previous_angle_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    let previous_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            previous_angle_candle.id,
        )
        .unwrap();

    store.update_min_angle(previous_angle.id.clone()).unwrap();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    let general_corridor = vec![new_angle.props.candle.clone()];

    AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

    assert_eq!(store.get_min_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_max_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert_eq!(
        store
            .get_min_angle_before_bargaining_corridor()
            .unwrap()
            .unwrap(),
        previous_angle
    );

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_max_angle_and_new_angle_is_not_in_bargaining_corridor__should_update_max_angle(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

    assert_eq!(store.get_max_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_min_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_max_angle_and_new_angle_is_in_bargaining_corridor_and_previous_max_angle_does_not_exist__should_update_max_angle(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    let general_corridor = vec![new_angle.props.candle.clone()];

    AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

    assert_eq!(store.get_max_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_min_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_max_angle_and_new_angle_is_in_bargaining_corridor_and_previous_max_angle_exists_and_previous_max_angle_is_in_bargaining_corridor__should_update_max_angle(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let previous_angle_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    let previous_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            previous_angle_candle.id.clone(),
        )
        .unwrap();

    store.update_max_angle(previous_angle.id).unwrap();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    let general_corridor = vec![new_angle.props.candle.clone(), previous_angle_candle];

    AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

    assert_eq!(store.get_max_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_min_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());

    assert!(store
        .get_max_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn update_angles__new_real_max_angle_and_new_angle_is_in_bargaining_corridor_and_previous_max_angle_exists_and_previous_max_angle_is_not_in_bargaining_corridor__should_update_real_max_angle_and_max_angle_before_bargaining_corridor(
) {
    let mut store = InMemoryStepBacktestingStore::default();

    let previous_angle_candle = store
        .create_candle(xid::new().to_string(), Default::default())
        .unwrap();

    let previous_angle = store
        .create_angle(
            xid::new().to_string(),
            BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            previous_angle_candle.id,
        )
        .unwrap();

    store.update_max_angle(previous_angle.id.clone()).unwrap();

    let new_angle = Item {
        id: xid::new().to_string(),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store
                .create_candle(xid::new().to_string(), Default::default())
                .unwrap(),
        },
    };

    let general_corridor = vec![new_angle.props.candle.clone()];

    AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

    assert_eq!(store.get_max_angle().unwrap().unwrap(), new_angle);

    assert!(store.get_min_angle().unwrap().is_none());
    assert!(store.get_virtual_min_angle().unwrap().is_none());
    assert!(store.get_virtual_max_angle().unwrap().is_none());

    assert_eq!(
        store
            .get_max_angle_before_bargaining_corridor()
            .unwrap()
            .unwrap(),
        previous_angle
    );

    assert!(store
        .get_min_angle_before_bargaining_corridor()
        .unwrap()
        .is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__angles_do_not_exist__should_return_none() {
    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &None,
    };

    assert!(
        AngleUtilsImpl::get_crossed_angle(angles, &StepCandleProperties::default()).is_none()
    );
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__angles_exist_but_not_crossed__should_return_none() {
    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("1"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(Item {
            id: String::from("2"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("2"),
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
        }),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                open: dec!(1.38300),
                close: dec!(1.38500),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(AngleUtilsImpl::get_crossed_angle(angles, &current_candle).is_none());
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__min_angle_is_crossed_and_max_angle_does_not_exist__should_return_min_angle(
) {
    let min_angle = Item {
        id: String::from("2"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
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

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &None,
        min_angle: &Some(min_angle.clone()),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                open: dec!(1.38100),
                close: dec!(1.37999),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_crossed_angle(angles, &current_candle).unwrap(),
        &min_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__min_angle_is_crossed_and_max_angle_exists__should_return_min_angle() {
    let min_angle = Item {
        id: String::from("2"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
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

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("1"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(min_angle.clone()),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                open: dec!(1.38100),
                close: dec!(1.37999),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_crossed_angle(angles, &current_candle).unwrap(),
        &min_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__min_angle_is_crossed_by_gap__should_return_min_angle() {
    let min_angle = Item {
        id: String::from("2"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
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

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        max_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("1"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                high: dec!(1.39000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.39000),
                    },
                },
            },
        }),
        min_angle: &Some(min_angle.clone()),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                close: dec!(1.39000),
                open: dec!(1.37999),
                ..Default::default()
            },
            r#type: CandleType::Green,
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_crossed_angle(angles, &current_candle).unwrap(),
        &min_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__max_angle_is_crossed_and_min_angle_does_not_exist__should_return_max_angle(
) {
    let max_angle = Item {
        id: String::from("2"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
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

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        min_angle: &None,
        max_angle: &Some(max_angle.clone()),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                close: dec!(1.38001),
                open: dec!(1.37900),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_crossed_angle(angles, &current_candle).unwrap(),
        &max_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__max_angle_is_crossed_and_min_angle_exist__should_return_max_angle() {
    let max_angle = Item {
        id: String::from("2"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
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

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("1"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
        max_angle: &Some(max_angle.clone()),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                open: dec!(1.37900),
                close: dec!(1.38001),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_crossed_angle(angles, &current_candle).unwrap(),
        &max_angle
    );
}

#[test]
#[allow(non_snake_case)]
fn get_crossed_angle__max_angle_is_crossed_by_gap__should_return_max_angle() {
    let max_angle = Item {
        id: String::from("2"),
        props: FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
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

    let angles: MaxMinAngles<BasicAngleProperties, StepCandleProperties> = MaxMinAngles {
        min_angle: &Some(Item {
            id: String::from("1"),
            props: FullAngleProperties {
                base: BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                candle: Item {
                    id: String::from("1"),
                    props: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.37000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        leading_price: dec!(1.37000),
                    },
                },
            },
        }),
        max_angle: &Some(max_angle.clone()),
    };

    let current_candle = StepCandleProperties {
        base: BasicCandleProperties {
            prices: CandlePrices {
                close: dec!(1.37000),
                open: dec!(1.38001),
                ..Default::default()
            },
            r#type: CandleType::Red,
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(
        AngleUtilsImpl::get_crossed_angle(angles, &current_candle).unwrap(),
        &max_angle
    );
}
