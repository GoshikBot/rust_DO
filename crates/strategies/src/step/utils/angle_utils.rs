use crate::step::utils::entities::angle::{
    AngleId, AngleState, BasicAngleProperties, FullAngleProperties,
};
use crate::step::utils::entities::candle::StepCandleProperties;
use crate::step::utils::entities::{Diff, MaxMinAngles};
use crate::step::utils::stores::angle_store::StepAngleStore;
use anyhow::Result;
use base::entities::candle::CandleId;
use base::entities::{Item, Level};
use base::helpers::price_to_points;
use base::params::ParamOutputValue;
use std::cmp;
use std::cmp::Ordering;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct ExistingDiffs {
    pub current: Diff,
    pub previous: Diff,
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
        min_distance_between_new_and_current_max_and_min_angles: ParamOutputValue,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear: ParamOutputValue,
    ) -> Option<FullAngleProperties<BasicAngleProperties, C>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone,
        A: AsRef<BasicAngleProperties> + Debug + Clone;

    fn update_angles<A, C>(
        new_angle: Item<AngleId, FullAngleProperties<A, C>>,
        general_corridor: &[Item<CandleId, C>],
        angle_store: &mut impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
    ) -> Result<()>
    where
        A: AsRef<BasicAngleProperties> + Debug + Clone,
        C: AsRef<StepCandleProperties> + Debug + Clone + PartialEq;

    fn get_crossed_angle<'a, A, C>(
        angles: MaxMinAngles<'a, A, C>,
        current_candle: &C,
    ) -> Option<&'a Item<AngleId, FullAngleProperties<A, C>>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone,
        A: AsRef<BasicAngleProperties> + Debug + Clone;
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
        min_distance_between_new_and_current_angles: ParamOutputValue,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear: ParamOutputValue,
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
        min_distance_between_new_and_current_angles: ParamOutputValue,
        min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear: ParamOutputValue,
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
        new_angle: Item<AngleId, FullAngleProperties<A, C>>,
        general_corridor: &[Item<CandleId, C>],
        angle_store: &mut impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
    ) -> Result<()>
    where
        A: AsRef<BasicAngleProperties> + Debug + Clone,
        C: AsRef<StepCandleProperties> + Debug + Clone + PartialEq,
    {
        let new_angle = angle_store.create_angle(
            new_angle.id,
            new_angle.props.base,
            new_angle.props.candle.id,
        )?;

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

    fn get_crossed_angle<'a, A, C>(
        angles: MaxMinAngles<'a, A, C>,
        current_candle: &C,
    ) -> Option<&'a Item<AngleId, FullAngleProperties<A, C>>>
    where
        C: AsRef<StepCandleProperties> + Debug + Clone,
        A: AsRef<BasicAngleProperties> + Debug + Clone,
    {
        if let Some(min_angle) = angles.min_angle {
            // in case of the gap the min angle can be crossed by the current candle open price
            let current_candle_lowest_price = cmp::min(
                current_candle.as_ref().base.prices.open,
                current_candle.as_ref().base.prices.close,
            );

            if current_candle_lowest_price < min_angle.props.candle.props.as_ref().leading_price {
                return Some(min_angle);
            }
        }

        if let Some(max_angle) = angles.max_angle {
            // in case of the gap the max angle can be crossed by the current candle open price
            let current_candle_highest_price = cmp::max(
                current_candle.as_ref().base.prices.open,
                current_candle.as_ref().base.prices.close,
            );

            if current_candle_highest_price > max_angle.props.candle.props.as_ref().leading_price {
                return Some(max_angle);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests;
