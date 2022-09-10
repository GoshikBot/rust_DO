use crate::step::utils::entities::angle::{
    AngleId, AngleState, BasicAngleProperties, FullAngleProperties,
};
use crate::step::utils::entities::candle::StepCandleProperties;
use crate::step::utils::entities::Diff;
use crate::step::utils::stores::angle_store::StepAngleStore;
use anyhow::Result;
use base::entities::candle::CandleId;
use base::entities::{Item, Level};
use base::helpers::price_to_points;
use base::params::ParamValue;
use std::cmp::Ordering;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct ExistingDiffs {
    pub current: Diff,
    pub previous: Diff,
}

#[derive(Debug, Clone)]
pub struct MaxMinAngles<'a, A, C>
where
    C: AsRef<StepCandleProperties> + Debug + Clone,
    A: AsRef<BasicAngleProperties> + Debug + Clone,
{
    pub max_angle: &'a Option<Item<AngleId, FullAngleProperties<A, C>>>,
    pub min_angle: &'a Option<Item<AngleId, FullAngleProperties<A, C>>>,
}

impl<'a, A, C> Copy for MaxMinAngles<'a, A, C>
where
    A: AsRef<BasicAngleProperties> + Debug + Clone,
    C: AsRef<StepCandleProperties> + Debug + Clone,
{
}

pub trait AngleUtils {
    /// Calculates the difference between current and previous candle leading prices
    /// to further determine angles.
    fn get_diff_between_current_and_previous_candles<C>(
        current_candle_props: &C,
        previous_candle_props: &C,
    ) -> Diff
    where
        C: AsRef<StepCandleProperties>;

    /// Checks if a new angle has appeared and returns such an angle.
    fn get_new_angle<C, A>(
        previous_candle: &Item<CandleId, C>,
        diffs: ExistingDiffs,
        angles: MaxMinAngles<A, C>,
        min_distance_between_new_and_current_max_and_min_angles: ParamValue,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear: ParamValue,
    ) -> Option<FullAngleProperties<BasicAngleProperties, C>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone,
        A: AsRef<BasicAngleProperties> + Debug + Clone;

    fn update_angles<A, C>(
        new_angle: FullAngleProperties<A, C>,
        general_corridor: &[Item<CandleId, C>],
        angle_store: &mut impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
    ) -> Result<()>
    where
        A: AsRef<BasicAngleProperties> + Debug + Clone,
        C: AsRef<StepCandleProperties> + Debug + Clone + PartialEq;
}

pub struct AngleUtilsImpl;

impl AngleUtilsImpl {
    /// Checks if the current config matches the particular angle type.
    fn diffs_for_angle_type_exist(
        angle_type: Level,
        diffs: ExistingDiffs,
        previous_candle_props: &impl AsRef<StepCandleProperties>,
    ) -> bool {
        match angle_type {
            Level::Min => {
                diffs.current == Diff::Greater
                    && diffs.previous == Diff::Less
                    && previous_candle_props.as_ref().leading_price
                        == previous_candle_props.as_ref().base.prices.low
            }
            Level::Max => {
                diffs.current == Diff::Less
                    && diffs.previous == Diff::Greater
                    && previous_candle_props.as_ref().leading_price
                        == previous_candle_props.as_ref().base.prices.high
            }
        }
    }

    fn current_angle_is_crossed_by_new_one<C, A>(
        current_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        new_angle_candle_props: &C,
    ) -> bool
    where
        A: AsRef<BasicAngleProperties> + Debug + Clone,
        C: AsRef<StepCandleProperties> + Debug + Clone,
    {
        let current_angle_is_crossed = match current_angle.props.base.as_ref().r#type {
            Level::Max => {
                new_angle_candle_props.as_ref().leading_price
                    > current_angle.props.candle.props.as_ref().leading_price
            }
            Level::Min => {
                new_angle_candle_props.as_ref().leading_price
                    < current_angle.props.candle.props.as_ref().leading_price
            }
        };

        if current_angle_is_crossed {
            log::debug!(
                "current {:?} angle is crossed by the new angle: current angle: {:?},\
                 new angle candle: {:?}",
                current_angle.props.base.as_ref().r#type,
                current_angle,
                new_angle_candle_props,
            );
        } else {
            log::debug!(
                "current {:?} angle is NOT crossed by the new angle: current angle: {:?},\
                 new angle candle: {:?}",
                current_angle.props.base.as_ref().r#type,
                current_angle,
                new_angle_candle_props,
            );
        }

        current_angle_is_crossed
    }

    fn get_new_angle_of_type<A, C>(
        angle_type: Level,
        previous_candle: &Item<CandleId, C>,
        diffs: ExistingDiffs,
        angles: MaxMinAngles<A, C>,
        min_distance_between_new_and_current_angles: ParamValue,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear: ParamValue,
    ) -> Option<FullAngleProperties<BasicAngleProperties, C>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone,
        A: AsRef<BasicAngleProperties> + Debug + Clone,
    {
        let get_new_angle_if_angle_of_the_same_type_exists = || {
            if Self::current_angle_is_crossed_by_new_one(
                match angle_type {
                    Level::Max => angles.max_angle.as_ref().unwrap(),
                    Level::Min => angles.min_angle.as_ref().unwrap(),
                },
                &previous_candle.props,
            ) {
                log::debug!(
                    "{angle_type:?} angle is crossed by the new one,\
                    so the new {angle_type:?} angle can appear"
                );

                return Some(FullAngleProperties {
                    base: BasicAngleProperties {
                        r#type: angle_type,
                        state: AngleState::Real,
                    },
                    candle: previous_candle.clone(),
                });
            } else {
                log::debug!(
                    "{angle_type:?} angle is NOT crossed by the new one,\
                    so the new {angle_type:?} angle can't appear"
                );
            }

            None
        };

        let get_new_angle_if_angle_of_different_type_exists = || {
            let distance_between_new_and_current_angles = price_to_points(match angle_type {
                Level::Min => {
                    angles
                        .max_angle
                        .as_ref()
                        .unwrap()
                        .props
                        .candle
                        .props
                        .as_ref()
                        .leading_price
                        - previous_candle.props.as_ref().leading_price
                }
                Level::Max => {
                    previous_candle.props.as_ref().leading_price
                        - angles
                            .min_angle
                            .as_ref()
                            .unwrap()
                            .props
                            .candle
                            .props
                            .as_ref()
                            .leading_price
                }
            });

            if distance_between_new_and_current_angles
                >= min_distance_between_new_and_current_angles
            {
                log::debug!(
                    "distance between new and current angles is appropriate for the new {:?} angle \
                    to appear: {} >= {}",
                    angle_type,
                    distance_between_new_and_current_angles,
                    min_distance_between_new_and_current_angles
                );

                return Some(FullAngleProperties {
                    base: BasicAngleProperties {
                        r#type: angle_type,
                        state: AngleState::Real,
                    },
                    candle: previous_candle.clone(),
                });
            } else {
                log::debug!(
                    "distance between new and current angles is inappropriate for the new {:?} angle \
                    to appear: {} < {}",
                    angle_type,
                    distance_between_new_and_current_angles,
                    min_distance_between_new_and_current_angles
                );
            }

            None
        };

        if Self::diffs_for_angle_type_exist(angle_type, diffs, &previous_candle.props) {
            match angles {
                MaxMinAngles {
                    max_angle: None,
                    min_angle: Some(min_angle),
                } => {
                    log::debug!(
                        "max angle is None, min angle exists: min angle — {:?}",
                        min_angle
                    );

                    return match angle_type {
                        Level::Max => get_new_angle_if_angle_of_different_type_exists(),
                        Level::Min => get_new_angle_if_angle_of_the_same_type_exists(),
                    };
                }
                MaxMinAngles {
                    max_angle: Some(max_angle),
                    min_angle: None,
                } => {
                    return match angle_type {
                        Level::Max => get_new_angle_if_angle_of_the_same_type_exists(),
                        Level::Min => get_new_angle_if_angle_of_different_type_exists(),
                    }
                }
                MaxMinAngles {
                    max_angle: Some(max_angle),
                    min_angle: Some(min_angle),
                } => {
                    log::debug!(
                        "both min and max angles exist: min angle — {:?}, max angle — {:?}",
                        min_angle,
                        max_angle
                    );

                    if Self::current_angle_is_crossed_by_new_one(
                        match angle_type {
                            Level::Max => max_angle,
                            Level::Min => min_angle,
                        },
                        &previous_candle.props,
                    ) {
                        log::debug!(
                            "{angle_type:?} angle is crossed by the new angle, so the distance between max and min \
                            angles is still appropriate and the new {angle_type:?} angle can appear"
                        );

                        return Some(FullAngleProperties {
                            base: BasicAngleProperties {
                                r#type: angle_type,
                                state: AngleState::Real,
                            },
                            candle: previous_candle.clone(),
                        });
                    } else {
                        log::debug!(
                            "{angle_type:?} angle is NOT crossed by the new angle, so the extra checks \
                            should be performed"
                        );

                        let distance_between_new_and_current_angles =
                            price_to_points(match angle_type {
                                Level::Min => {
                                    max_angle.props.candle.props.as_ref().leading_price
                                        - previous_candle.props.as_ref().leading_price
                                }
                                Level::Max => {
                                    previous_candle.props.as_ref().leading_price
                                        - min_angle.props.candle.props.as_ref().leading_price
                                }
                            });

                        if distance_between_new_and_current_angles
                            >= min_distance_between_new_and_current_angles
                        {
                            log::debug!(
                                "distance between new and current angles is appropriate for the new {angle_type:?} angle \
                                 to appear: ({}) >= ({})",
                                distance_between_new_and_current_angles,
                                min_distance_between_new_and_current_angles
                            );

                            let distance_between_current_max_min_angles = price_to_points(
                                max_angle.props.candle.props.as_ref().leading_price
                                    - min_angle.props.candle.props.as_ref().leading_price,
                            );

                            return if distance_between_current_max_min_angles >= min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear {
                                log::debug!(
                                    "distance between current max and min angles is appropriate for the new \
                                    real inner angle to appear: {} >= {}",
                                    distance_between_current_max_min_angles,
                                    min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
                                );

                                Some(FullAngleProperties {
                                    base: BasicAngleProperties {
                                        r#type: angle_type,
                                        state: AngleState::Real,
                                    },
                                    candle: previous_candle.clone(),
                                })
                            } else {
                                log::debug!(
                                    "distance between current max and min angles is inappropriate for the new \
                                    {angle_type:?} real inner angle to appear, but is appropriate for the new \
                                    {angle_type:?} virtual inner angle: {} < {}",
                                    distance_between_current_max_min_angles,
                                    min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear
                                );

                                Some(FullAngleProperties {
                                    base: BasicAngleProperties {
                                        r#type: angle_type,
                                        state: AngleState::Virtual,
                                    },
                                    candle: previous_candle.clone(),
                                })
                            };
                        } else {
                            log::debug!(
                                "distance between new and current angles is inappropriate for \
                                the new {angle_type:?} angle to appear: {} < {}",
                                distance_between_new_and_current_angles,
                                min_distance_between_new_and_current_angles
                            );
                        }
                    }
                }
                MaxMinAngles {
                    max_angle: None,
                    min_angle: None,
                } => {
                    log::debug!(
                        "neither min nor max angle exist, so the new max angle can appear \
                        without any extra checks"
                    );

                    return Some(FullAngleProperties {
                        base: BasicAngleProperties {
                            r#type: angle_type,
                            state: AngleState::Real,
                        },
                        candle: previous_candle.clone(),
                    });
                }
            }
        }

        None
    }

    fn get_angle_before_bargaining_corridor<'a, A, C>(
        new_angle: &FullAngleProperties<A, C>,
        general_corridor: &[Item<CandleId, C>],
        angles: MaxMinAngles<'a, A, C>,
    ) -> Option<&'a Item<AngleId, FullAngleProperties<A, C>>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone + PartialEq,
        A: AsRef<BasicAngleProperties> + Debug + Clone,
    {
        if general_corridor.contains(&new_angle.candle) {
            log::debug!(
                "new angle candle is in the supposed bargaining corridor:\
                candle — {:?}, bargaining corridor — {:?}",
                new_angle.candle,
                general_corridor
            );

            match new_angle.base.as_ref().r#type {
                Level::Min => {
                    if let Some(min_angle) = angles.min_angle {
                        if !general_corridor.contains(&min_angle.props.candle) {
                            log::debug!(
                                "the previous min angle is not the supposed bargaining corridor,\
                                so the previous min angle can be the angle before the bargaining corridor:\
                                previous min angle — {:?}, bargaining corridor — {:?}",
                                min_angle,
                                general_corridor
                            );

                            return Some(min_angle);
                        } else {
                            log::debug!(
                                "the previous min angle is in the supposed bargaining corridor,\
                                so the previous min angle cannot be the angle before the bargaining corridor:\
                                previous min angle — {:?}, bargaining corridor — {:?}",
                                min_angle,
                                general_corridor
                            );
                        }
                    } else {
                        log::debug!(
                            "the previous min angle is None, it can't be considered as the angle \
                            before the bargaining corridor"
                        );
                    }
                }
                Level::Max => {
                    if let Some(max_angle) = angles.max_angle {
                        if !general_corridor.contains(&max_angle.props.candle) {
                            log::debug!(
                                "the previous max angle is not the supposed bargaining corridor,\
                                so the previous max angle can be the angle before the bargaining corridor:\
                                previous max angle — {:?}, bargaining corridor — {:?}",
                                max_angle,
                                general_corridor
                            );

                            return Some(max_angle);
                        } else {
                            log::debug!(
                                "the previous max angle is in the supposed bargaining corridor,\
                                so the previous max angle cannot be the angle before the bargaining corridor:\
                                previous max angle — {:?}, bargaining corridor — {:?}",
                                max_angle,
                                general_corridor
                            );
                        }
                    } else {
                        log::debug!(
                            "the previous max angle is None, it can't be considered as the angle \
                            before the bargaining corridor"
                        );
                    }
                }
            }
        }

        None
    }
}

impl AngleUtils for AngleUtilsImpl {
    fn get_diff_between_current_and_previous_candles<C>(
        current_candle_props: &C,
        previous_candle_props: &C,
    ) -> Diff
    where
        C: AsRef<StepCandleProperties>,
    {
        let current_candle_props = current_candle_props.as_ref();
        let previous_candle_props = previous_candle_props.as_ref();

        match current_candle_props
            .leading_price
            .cmp(&previous_candle_props.leading_price)
        {
            Ordering::Greater => Diff::Greater,
            Ordering::Less => Diff::Less,
            Ordering::Equal => {
                if current_candle_props.leading_price == current_candle_props.base.prices.high {
                    Diff::Greater
                } else {
                    Diff::Less
                }
            }
        }
    }

    fn get_new_angle<C, A>(
        previous_candle: &Item<CandleId, C>,
        diffs: ExistingDiffs,
        angles: MaxMinAngles<A, C>,
        min_distance_between_new_and_current_angles: ParamValue,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear: ParamValue,
    ) -> Option<FullAngleProperties<BasicAngleProperties, C>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone,
        A: AsRef<BasicAngleProperties> + Debug + Clone,
    {
        let new_max_angle = Self::get_new_angle_of_type(
            Level::Max,
            previous_candle,
            diffs,
            angles,
            min_distance_between_new_and_current_angles,
            min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear,
        );

        match new_max_angle {
            Some(_) => new_max_angle,
            None => Self::get_new_angle_of_type(
                Level::Min,
                previous_candle,
                diffs,
                angles,
                min_distance_between_new_and_current_angles,
                min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear,
            ),
        }
    }

    fn update_angles<A, C>(
        new_angle: FullAngleProperties<A, C>,
        general_corridor: &[Item<CandleId, C>],
        angle_store: &mut impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
    ) -> Result<()>
    where
        A: AsRef<BasicAngleProperties> + Debug + Clone,
        C: AsRef<StepCandleProperties> + Debug + Clone + PartialEq,
    {
        let new_angle = angle_store.create_angle(new_angle.base, new_angle.candle.id)?;

        match new_angle.props.base.as_ref().state {
            AngleState::Real => {
                let max_angle = angle_store.get_max_angle()?;
                let min_angle = angle_store.get_min_angle()?;

                let new_angle_before_bargaining_corridor =
                    Self::get_angle_before_bargaining_corridor(
                        &new_angle.props,
                        general_corridor,
                        MaxMinAngles {
                            max_angle: &max_angle,
                            min_angle: &min_angle,
                        },
                    );

                if let Some(new_angle_before_bargaining_corridor) =
                    new_angle_before_bargaining_corridor
                {
                    match new_angle_before_bargaining_corridor
                        .props
                        .base
                        .as_ref()
                        .r#type
                    {
                        Level::Min => angle_store.update_min_angle_before_bargaining_corridor(
                            new_angle_before_bargaining_corridor.id.clone(),
                        )?,
                        Level::Max => angle_store.update_max_angle_before_bargaining_corridor(
                            new_angle_before_bargaining_corridor.id.clone(),
                        )?,
                    }
                }

                match new_angle.props.base.as_ref().r#type {
                    Level::Min => angle_store.update_min_angle(new_angle.id.clone())?,
                    Level::Max => angle_store.update_max_angle(new_angle.id.clone())?,
                }
            }
            AngleState::Virtual => match new_angle.props.base.as_ref().r#type {
                Level::Min => angle_store.update_virtual_min_angle(new_angle.id)?,
                Level::Max => angle_store.update_virtual_max_angle(new_angle.id)?,
            },
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
    use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use crate::step::utils::stores::StepDiffs;
    use base::entities::candle::BasicCandleProperties;
    use base::entities::CandlePrices;
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

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Virtual,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

        assert_eq!(
            store.get_virtual_min_angle().unwrap().unwrap().props,
            new_angle
        );

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

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Virtual,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

        assert_eq!(
            store.get_virtual_max_angle().unwrap().unwrap().props,
            new_angle
        );

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

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

        assert_eq!(store.get_min_angle().unwrap().unwrap().props, new_angle);

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

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        let general_corridor = vec![new_angle.candle.clone()];

        AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

        assert_eq!(store.get_min_angle().unwrap().unwrap().props, new_angle);

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

        let previous_angle_candle = store.create_candle(Default::default()).unwrap();

        let previous_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                previous_angle_candle.id.clone(),
            )
            .unwrap();

        store.update_min_angle(previous_angle.id).unwrap();

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        let general_corridor = vec![new_angle.candle.clone(), previous_angle_candle];

        AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

        assert_eq!(store.get_min_angle().unwrap().unwrap().props, new_angle);

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

        let previous_angle_candle = store.create_candle(Default::default()).unwrap();

        let previous_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    state: AngleState::Real,
                },
                previous_angle_candle.id,
            )
            .unwrap();

        store.update_min_angle(previous_angle.id.clone()).unwrap();

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        let general_corridor = vec![new_angle.candle.clone()];

        AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

        assert_eq!(store.get_min_angle().unwrap().unwrap().props, new_angle);

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

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        AngleUtilsImpl::update_angles(new_angle.clone(), &Vec::new(), &mut store).unwrap();

        assert_eq!(store.get_max_angle().unwrap().unwrap().props, new_angle);

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

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        let general_corridor = vec![new_angle.candle.clone()];

        AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

        assert_eq!(store.get_max_angle().unwrap().unwrap().props, new_angle);

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

        let previous_angle_candle = store.create_candle(Default::default()).unwrap();

        let previous_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                previous_angle_candle.id.clone(),
            )
            .unwrap();

        store.update_max_angle(previous_angle.id).unwrap();

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        let general_corridor = vec![new_angle.candle.clone(), previous_angle_candle];

        AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

        assert_eq!(store.get_max_angle().unwrap().unwrap().props, new_angle);

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

        let previous_angle_candle = store.create_candle(Default::default()).unwrap();

        let previous_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Max,
                    state: AngleState::Real,
                },
                previous_angle_candle.id,
            )
            .unwrap();

        store.update_max_angle(previous_angle.id.clone()).unwrap();

        let new_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: store.create_candle(Default::default()).unwrap(),
        };

        let general_corridor = vec![new_angle.candle.clone()];

        AngleUtilsImpl::update_angles(new_angle.clone(), &general_corridor, &mut store).unwrap();

        assert_eq!(store.get_max_angle().unwrap().unwrap().props, new_angle);

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
}
