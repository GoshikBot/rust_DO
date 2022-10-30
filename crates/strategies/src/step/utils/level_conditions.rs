use crate::step::utils::entities::angle::{AngleId, BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::StepCandleProperties;
use crate::step::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::entities::working_levels::{
    BasicWLProperties, CorridorType, LevelTime, WLId, WLMaxCrossingValue, WLPrice,
};
use crate::step::utils::entities::MaxMinAngles;
use crate::step::utils::stores::angle_store::StepAngleStore;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::candle::CandleId;
use base::entities::order::{BasicOrderProperties, OrderPrice, OrderStatus, OrderType};
use base::entities::tick::{TickPrice, TickTime, UniversalTickPrice};
use base::entities::{Item, Level, DEFAULT_HOLIDAYS};
use base::helpers::{price_to_points, Holiday, NumberOfDaysToExclude};
use base::params::{ParamOutputValue, StrategyParams};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp;
use std::fmt::Debug;

pub type MinAmountOfCandles = ParamOutputValue;

pub trait LevelConditions {
    /// Checks whether the level exceeds the amount of candles in the corridor
    /// before the activation crossing of the level.
    fn level_exceeds_amount_of_candles_in_corridor(
        level_id: &str,
        working_level_store: &impl StepWorkingLevelStore,
        corridor_type: CorridorType,
        min_amount_of_candles: MinAmountOfCandles,
    ) -> Result<bool>;

    fn price_is_beyond_stop_loss(
        current_tick_price: UniversalTickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool;

    fn level_expired_by_distance(
        level_price: WLPrice,
        current_tick_price: UniversalTickPrice,
        distance_from_level_for_its_deletion: ParamOutputValue,
    ) -> bool;

    fn level_expired_by_time(
        level_time: LevelTime,
        current_tick_time: TickTime,
        level_expiration: ParamOutputValue,
        exclude_weekend_and_holidays: &impl Fn(
            NaiveDateTime,
            NaiveDateTime,
            &[Holiday],
        ) -> NumberOfDaysToExclude,
    ) -> bool;

    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        level: &impl AsRef<BasicWLProperties>,
        max_crossing_value: Option<WLMaxCrossingValue>,
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamOutputValue,
        current_tick_price: UniversalTickPrice,
    ) -> bool;

    fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool;

    fn is_second_level_after_bargaining_tendency_change(
        crossed_angle: &str,
        tendency_change_angle: Option<&str>,
        last_tendency_changed_on_crossing_bargaining_corridor: bool,
        second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool;

    fn level_comes_out_of_bargaining_corridor<A, C>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        general_corridor: &[Item<CandleId, C>],
        angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq;

    fn appropriate_working_level<A, C>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        current_candle: &Item<CandleId, C>,
        angle_store: &impl StepAngleStore<AngleProperties = A, CandleProperties = C>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug;

    fn working_level_exists<A, C, W>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties>;

    fn working_level_is_close_to_another_one<A, C, W>(
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        working_level_store: &impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        distance_defining_nearby_levels_of_the_same_type: ParamOutputValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug,
        W: AsRef<BasicWLProperties> + Debug;
}

#[derive(Default)]
pub struct LevelConditionsImpl;

impl LevelConditions for LevelConditionsImpl {
    fn level_exceeds_amount_of_candles_in_corridor(
        level_id: &str,
        working_level_store: &impl StepWorkingLevelStore,
        corridor_type: CorridorType,
        min_amount_of_candles: MinAmountOfCandles,
    ) -> Result<bool> {
        let corridor =
            working_level_store.get_candles_of_working_level_corridor(level_id, corridor_type)?;

        Ok(ParamOutputValue::from(corridor.len()) >= min_amount_of_candles)
    }

    fn price_is_beyond_stop_loss(
        current_tick_price: UniversalTickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool {
        let (lowest_tick_price, highest_tick_price) = match current_tick_price {
            UniversalTickPrice::Historical(current_tick_price) => {
                (current_tick_price.low, current_tick_price.high)
            }
            UniversalTickPrice::Realtime(current_tick_price) => {
                (current_tick_price, current_tick_price)
            }
        };

        (working_level_type == OrderType::Buy && lowest_tick_price <= stop_loss_price)
            || working_level_type == OrderType::Sell && highest_tick_price >= stop_loss_price
    }

    fn level_expired_by_distance(
        level_price: WLPrice,
        current_tick_price: UniversalTickPrice,
        distance_from_level_for_its_deletion: ParamOutputValue,
    ) -> bool {
        log::debug!(
            "level_expired_by_distance: level price is {}, current tick price is {:?}, \
            distance from level for its deletion is {}",
            level_price,
            current_tick_price,
            distance_from_level_for_its_deletion
        );

        let (lowest_tick_price, highest_tick_price) = match current_tick_price {
            UniversalTickPrice::Historical(current_tick_price) => {
                (current_tick_price.low, current_tick_price.high)
            }
            UniversalTickPrice::Realtime(current_tick_price) => {
                (current_tick_price, current_tick_price)
            }
        };

        let max_distance = cmp::max(
            (level_price - lowest_tick_price).abs(),
            (level_price - highest_tick_price).abs(),
        );

        price_to_points(max_distance) >= distance_from_level_for_its_deletion
    }

    fn level_expired_by_time(
        level_time: LevelTime,
        current_tick_time: TickTime,
        level_expiration: ParamOutputValue,
        exclude_weekend_and_holidays: &impl Fn(
            NaiveDateTime,
            NaiveDateTime,
            &[Holiday],
        ) -> NumberOfDaysToExclude,
    ) -> bool {
        let diff = (current_tick_time - level_time).num_days()
            - exclude_weekend_and_holidays(level_time, current_tick_time, &DEFAULT_HOLIDAYS) as i64;

        log::debug!(
            "level_expired_by_time: current tick time is {}, level time is {},\
            level expiration is {}, diff is {}",
            current_tick_time,
            level_time,
            level_expiration,
            diff
        );

        Decimal::from(diff) >= level_expiration
    }

    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        level: &impl AsRef<BasicWLProperties>,
        max_crossing_value: Option<WLMaxCrossingValue>,
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamOutputValue,
        current_tick_price: UniversalTickPrice,
    ) -> bool {
        let level = level.as_ref();

        let (lowest_tick_price, highest_tick_price) = match current_tick_price {
            UniversalTickPrice::Historical(current_tick_price) => {
                (current_tick_price.low, current_tick_price.high)
            }
            UniversalTickPrice::Realtime(current_tick_price) => {
                (current_tick_price, current_tick_price)
            }
        };

        if (level.r#type == OrderType::Buy && highest_tick_price >= level.price)
            || (level.r#type == OrderType::Sell && lowest_tick_price <= level.price)
        {
            if let Some(max_crossing_value) = max_crossing_value {
                if max_crossing_value >= min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion {
                    return true;
                }
            }
        }

        false
    }

    fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
        for order in level_orders {
            if order.as_ref().status != OrderStatus::Pending {
                return false;
            }
        }

        true
    }

    fn is_second_level_after_bargaining_tendency_change(
        crossed_angle: &str,
        tendency_change_angle: Option<&str>,
        last_tendency_changed_on_crossing_bargaining_corridor: bool,
        second_level_after_bargaining_tendency_change_is_created: bool,
    ) -> bool {
        if let Some(tendency_change_angle) = tendency_change_angle {
            if last_tendency_changed_on_crossing_bargaining_corridor
                && !second_level_after_bargaining_tendency_change_is_created
                && crossed_angle != tendency_change_angle
            {
                log::debug!("it's the second level after bargaining tendency change");
                return true;
            }
        }

        log::debug!("it's NOT the second level after bargaining tendency change");
        false
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
        if ParamOutputValue::from(general_corridor.len())
            >= min_amount_of_candles_in_corridor_defining_edge_bargaining
        {
            log::debug!(
                "len of general corridor >= min amount of candles in corridor defining edge bargaining:\
                general corridor — {general_corridor:?}, min amount of candles in corridor defining \
                edge bargaining — {min_amount_of_candles_in_corridor_defining_edge_bargaining}",
            );

            let max_angle = angle_store.get_max_angle()?;
            let min_angle = angle_store.get_min_angle()?;

            if let (Some(min_angle), Some(max_angle)) = (&min_angle, &max_angle) {
                if general_corridor.contains(&min_angle.props.candle)
                    && general_corridor.contains(&max_angle.props.candle)
                {
                    log::debug!(
                        "min angle and max angle are in corridor defining edge bargaining: min angle — {min_angle:?},\
                        max angle — {max_angle:?}, general corridor — {general_corridor:?}"
                    );

                    match crossed_angle.props.base.as_ref().r#type {
                        Level::Min => {
                            if let Some(max_angle_before_bargaining_corridor) =
                                angle_store.get_max_angle_before_bargaining_corridor()?
                            {
                                if max_angle_before_bargaining_corridor
                                    .props
                                    .candle
                                    .props
                                    .as_ref()
                                    .leading_price
                                    < max_angle.props.candle.props.as_ref().leading_price
                                {
                                    log::debug!(
                                        "max angle before bargaining corridor is less than current max angle inside bargaining \
                                        corridor, so the current level comes out of bargaining corridor: max angle before \
                                        bargaining corridor — {max_angle_before_bargaining_corridor:?},\
                                        current max angle inside bargaining corridor — {max_angle:?}"
                                    );

                                    return Ok(true);
                                } else {
                                    log::debug!(
                                        "max angle before bargaining corridor is greater than current max angle inside \
                                        bargaining corridor, so the current level doesn't come out of bargaining corridor: \
                                        max angle before bargaining corridor — {max_angle_before_bargaining_corridor:?},\
                                        current max angle inside bargaining corridor — {max_angle:?}"
                                    );
                                }
                            } else {
                                log::debug!(
                                    "max angle before bargaining corridor is None, so it's impossible to determine whether \
                                    the current level comes out of bargaining corridor or not"
                                );
                            }
                        }
                        Level::Max => {
                            if let Some(min_angle_before_bargaining_corridor) =
                                angle_store.get_min_angle_before_bargaining_corridor()?
                            {
                                if min_angle_before_bargaining_corridor
                                    .props
                                    .candle
                                    .props
                                    .as_ref()
                                    .leading_price
                                    > min_angle.props.candle.props.as_ref().leading_price
                                {
                                    log::debug!(
                                        "min angle before bargaining corridor is greater than current min angle inside \
                                        bargaining corridor, so the current level comes out of bargaining corridor: min \
                                        angle before bargaining corridor — {min_angle_before_bargaining_corridor:?},\
                                        current min angle inside bargaining corridor — {min_angle:?}"
                                    );

                                    return Ok(true);
                                } else {
                                    log::debug!(
                                        "min angle before bargaining corridor is less than current min angle inside \
                                        bargaining corridor, so the current level doesn't come out of bargaining corridor: \
                                        min angle before bargaining corridor — {min_angle_before_bargaining_corridor:?},\
                                        current min angle inside bargaining corridor — {min_angle:?}"
                                    );
                                }
                            } else {
                                log::debug!(
                                    "min angle before bargaining corridor is None, so it's impossible to determine whether \
                                    the current level comes out of bargaining corridor or not"
                                );
                            }
                        }
                    }
                } else {
                    log::debug!(
                        "either min or max angle is NOT in corridor defining edge bargaining, so \
                        the current level doesn't come out of bargaining corridor:\
                        min angle — {min_angle:?}, max angle — {max_angle:?}, general corridor — {general_corridor:?}"
                    );
                }
            } else {
                log::debug!(
                    "either min or max angle is None, so it's impossible to determine whether \
                    the current level comes out of bargaining corridor: min angle — {min_angle:?}, \
                    max angle — {max_angle:?}"
                );
            }
        } else {
            log::debug!(
                "len of general corridor < min amount of candles in corridor defining edge bargaining:\
                general corridor — {general_corridor:?}, min amount of candles in corridor defining \
                edge bargaining — {min_amount_of_candles_in_corridor_defining_edge_bargaining}",
            );
        }

        Ok(false)
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
        let min_angle = angle_store.get_min_angle()?;
        let max_angle = angle_store.get_max_angle()?;

        match (min_angle, max_angle) {
            (Some(min_angle), Some(max_angle)) => {
                let min_break_distance = params.get_ratio_param_value(
                    StepRatioParam::MinBreakDistance,
                    current_candle.props.as_ref().base.volatility,
                );

                match crossed_angle.props.base.as_ref().r#type {
                    Level::Min => {
                        let current_candle_lowest_price = cmp::min(
                            current_candle.props.as_ref().base.prices.open,
                            current_candle.props.as_ref().base.prices.close,
                        );

                        let break_distance = price_to_points(
                            crossed_angle.props.candle.props.as_ref().leading_price
                                - current_candle_lowest_price,
                        );

                        if break_distance >= min_break_distance {
                            if max_angle.props.candle.props.as_ref().base.time
                                > min_angle.props.candle.props.as_ref().base.time
                            {
                                log::debug!(
                                    "the max angle time is later than the min angle time, so the \
                                    current level is appropriate working level: max angle — {max_angle:?}, \
                                    min angle — {min_angle:?}"
                                );

                                return Ok(true);
                            } else {
                                log::debug!(
                                    "the max angle time is earlier than the min angle time, so the \
                                    extra checks are required: max angle — {max_angle:?}, \
                                    min angle — {min_angle:?}"
                                );

                                if let Some(virtual_max_angle) =
                                    angle_store.get_virtual_max_angle()?
                                {
                                    if virtual_max_angle.props.candle.props.as_ref().base.time
                                        > min_angle.props.candle.props.as_ref().base.time
                                    {
                                        log::debug!(
                                            "the virtual max angle time is later than the min angle time, so the \
                                            current level is appropriate working level: virtual max angle — \
                                            {virtual_max_angle:?}, min angle — {min_angle:?}"
                                        );

                                        return Ok(true);
                                    } else {
                                        log::debug!(
                                            "the virtual max angle time is earlier than the min angle time, so the \
                                            extra checks are required to determine whether the current level is appropriate:\
                                            virtual max angle — {virtual_max_angle:?}, min angle — {min_angle:?}"
                                        );
                                    }
                                } else {
                                    log::debug!(
                                        "virtual max angle is None, so the extra checks are required to determine \
                                        whether the current level is appropriate"
                                    );
                                }

                                let min_distance_between_max_and_min_angles = params
                                    .get_ratio_param_value(
                                        StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles,
                                        current_candle.props.as_ref().base.volatility,
                                    );

                                let distance_between_current_candle_and_min_angle = price_to_points(
                                    current_candle.props.as_ref().base.prices.high
                                        - min_angle.props.candle.props.as_ref().leading_price,
                                );

                                if distance_between_current_candle_and_min_angle
                                    >= min_distance_between_max_and_min_angles
                                {
                                    log::debug!(
                                        "the min distance between the current candle high and the min angle is present,\
                                        so the current level is appropriate: distance between current candle and min angle — \
                                        {distance_between_current_candle_and_min_angle}, min distance between max and min angles — \
                                        {min_distance_between_max_and_min_angles}, current candle — {current_candle:?}, \
                                        min angle — {min_angle:?}",
                                    );

                                    return Ok(true);
                                } else {
                                    log::debug!(
                                        "the min distance between the current candle high and the min angle is NOT present,\
                                        so the current level is NOT appropriate: distance between current candle and min angle — \
                                        {distance_between_current_candle_and_min_angle}, min distance between max and min angles — \
                                        {min_distance_between_max_and_min_angles}, current candle — {current_candle:?}, \
                                        min angle — {min_angle:?}",
                                    );
                                }
                            }
                        } else {
                            log::debug!(
                                "min break distance is inappropriate for the new working level to appear:\
                                break distance — {break_distance}, min break distance — {min_break_distance},\
                                current candle — {current_candle:?}, crossed angle — {crossed_angle:?}",
                            )
                        }
                    }
                    Level::Max => {
                        let current_candle_highest_price = cmp::max(
                            current_candle.props.as_ref().base.prices.open,
                            current_candle.props.as_ref().base.prices.close,
                        );

                        let break_distance = price_to_points(
                            current_candle_highest_price
                                - crossed_angle.props.candle.props.as_ref().leading_price,
                        );

                        if break_distance >= min_break_distance {
                            if min_angle.props.candle.props.as_ref().base.time
                                > max_angle.props.candle.props.as_ref().base.time
                            {
                                log::debug!(
                                    "the min angle time is later than the max angle time, so the \
                                    current level is appropriate working level: min angle — {min_angle:?}, \
                                    max angle — {max_angle:?}"
                                );

                                return Ok(true);
                            } else {
                                log::debug!(
                                    "the min angle time is earlier than the max angle time, so the \
                                    extra checks are required: min angle — {min_angle:?}, \
                                    max angle — {max_angle:?}"
                                );

                                if let Some(virtual_min_angle) =
                                    angle_store.get_virtual_min_angle()?
                                {
                                    if virtual_min_angle.props.candle.props.as_ref().base.time
                                        > max_angle.props.candle.props.as_ref().base.time
                                    {
                                        log::debug!(
                                            "the virtual min angle time is later than the max angle time, so the \
                                            current level is appropriate working level: virtual min angle — \
                                            {virtual_min_angle:?}, max angle — {max_angle:?}"
                                        );

                                        return Ok(true);
                                    } else {
                                        log::debug!(
                                            "the virtual min angle time is earlier than the max angle time, so the \
                                            extra checks are required to determine whether the current level is appropriate:\
                                            virtual min angle — {virtual_min_angle:?}, max angle — {max_angle:?}"
                                        );
                                    }
                                } else {
                                    log::debug!(
                                        "virtual min angle is None, so the extra checks are required to determine \
                                        whether the current level is appropriate"
                                    );
                                }

                                let min_distance_between_max_and_min_angles = params
                                    .get_ratio_param_value(
                                        StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles,
                                        current_candle.props.as_ref().base.volatility,
                                    );

                                let distance_between_current_candle_and_max_angle = price_to_points(
                                    max_angle.props.candle.props.as_ref().leading_price
                                        - current_candle.props.as_ref().base.prices.low,
                                );

                                if distance_between_current_candle_and_max_angle
                                    >= min_distance_between_max_and_min_angles
                                {
                                    log::debug!(
                                        "the min distance between the current candle low and the max angle is present,\
                                        so the current level is appropriate: distance between current candle and max angle — \
                                        {distance_between_current_candle_and_max_angle}, min distance between max and min angles — \
                                        {min_distance_between_max_and_min_angles}, current candle — {current_candle:?}, \
                                        max angle — {max_angle:?}",
                                    );

                                    return Ok(true);
                                } else {
                                    log::debug!(
                                        "the min distance between the current candle low and the max angle is NOT present,\
                                        so the current level is NOT appropriate: distance between current candle and max angle — \
                                        {distance_between_current_candle_and_max_angle}, min distance between max and min angles — \
                                        {min_distance_between_max_and_min_angles}, current candle — {current_candle:?}, \
                                        max angle — {max_angle:?}",
                                    );
                                }
                            }
                        } else {
                            log::debug!(
                                "min break distance is inappropriate for the new working level to appear:\
                                break distance — {break_distance}, min break distance — {min_break_distance},\
                                current candle — {current_candle:?}, crossed angle — {crossed_angle:?}",
                            )
                        }
                    }
                }
            }
            (min_angle, max_angle) => {
                log::debug!(
                    "either min or max angle is None, so it's impossible to determine whether \
                    the current level is appropriate working level: min angle — {min_angle:?}, \
                    max angle — {max_angle:?}"
                );
            }
        }

        Ok(false)
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
        let level_is_present_in_created_working_levels = working_level_store
            .get_created_working_levels()?
            .iter()
            .any(|level| {
                level.props.as_ref().price
                    == crossed_angle.props.candle.props.as_ref().leading_price
                    && level.props.as_ref().time
                        == crossed_angle.props.candle.props.as_ref().base.time
            });

        if level_is_present_in_created_working_levels {
            log::debug!(
                "the working level on the current crossed angle is already present in the created \
                working levels: crossed angle — {crossed_angle:?}"
            );

            return Ok(true);
        }

        let level_is_present_in_active_working_levels = working_level_store
            .get_active_working_levels()?
            .iter()
            .any(|level| {
                level.props.as_ref().price
                    == crossed_angle.props.candle.props.as_ref().leading_price
                    && level.props.as_ref().time
                        == crossed_angle.props.candle.props.as_ref().base.time
            });

        if level_is_present_in_active_working_levels {
            log::debug!(
                "the working level on the current crossed angle is already present in the active \
                working levels: crossed angle — {crossed_angle:?}"
            );

            return Ok(true);
        }

        log::debug!(
            "the working level on the current crossed angle is NOT present neither in the created \
            nor in the active working levels: crossed angle — {crossed_angle:?}"
        );

        Ok(false)
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
        for existing_level in working_level_store
            .get_created_working_levels()?
            .iter()
            .chain(working_level_store.get_active_working_levels()?.iter())
        {
            if OrderType::from(crossed_angle.props.base.as_ref().r#type)
                == existing_level.props.as_ref().r#type
            {
                let distance_between_levels = price_to_points(
                    match OrderType::from(crossed_angle.props.base.as_ref().r#type) {
                        OrderType::Buy => {
                            crossed_angle.props.candle.props.as_ref().leading_price
                                - existing_level.props.as_ref().price
                        }
                        OrderType::Sell => {
                            existing_level.props.as_ref().price
                                - crossed_angle.props.candle.props.as_ref().leading_price
                        }
                    },
                );

                if distance_between_levels >= dec!(0)
                    && distance_between_levels <= distance_defining_nearby_levels_of_the_same_type
                {
                    log::debug!(
                        "the new level is close to the existing level: distance between levels — \
                        {distance_between_levels}, distance defining nearby levels of the same type — \
                        {distance_defining_nearby_levels_of_the_same_type}, crossed angle — {crossed_angle:?}, \
                        existing level — {existing_level:?}",
                    );

                    return Ok(true);
                } else {
                    log::debug!(
                        "the new level is NOT close to the existing level: distance between levels — \
                        {distance_between_levels}, distance defining nearby levels of the same type — \
                        {distance_defining_nearby_levels_of_the_same_type}, crossed angle — {crossed_angle:?}, \
                        existing level — {existing_level:?}",
                    );
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests;