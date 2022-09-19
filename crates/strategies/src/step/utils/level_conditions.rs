use crate::step::utils::entities::angle::{AngleId, BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::StepCandleProperties;
use crate::step::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::entities::working_levels::{
    BasicWLProperties, CorridorType, LevelTime, WLMaxCrossingValue, WLPrice,
};
use crate::step::utils::entities::MaxMinAngles;
use crate::step::utils::stores::angle_store::StepAngleStore;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::candle::CandleId;
use base::entities::order::{BasicOrderProperties, OrderPrice, OrderStatus, OrderType};
use base::entities::tick::{TickPrice, TickTime};
use base::entities::{Item, Level, DEFAULT_HOLIDAYS};
use base::helpers::{price_to_points, Holiday, NumberOfDaysToExclude};
use base::params::{ParamValue, StrategyParams};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use std::cmp;
use std::fmt::Debug;

pub type MinAmountOfCandles = ParamValue;

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
        current_tick_price: TickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool;

    fn level_expired_by_distance(
        level_price: WLPrice,
        current_tick_price: TickPrice,
        distance_from_level_for_its_deletion: ParamValue,
    ) -> bool;

    fn level_expired_by_time(
        level_time: LevelTime,
        current_tick_time: TickTime,
        level_expiration: ParamValue,
        exclude_weekend_and_holidays: &impl Fn(
            NaiveDateTime,
            NaiveDateTime,
            &[Holiday],
        ) -> NumberOfDaysToExclude,
    ) -> bool;

    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        level: &impl AsRef<BasicWLProperties>,
        max_crossing_value: Option<WLMaxCrossingValue>,
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
        current_tick_price: TickPrice,
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
        min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamValue,
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

        Ok(ParamValue::from(corridor.len()) >= min_amount_of_candles)
    }

    fn price_is_beyond_stop_loss(
        current_tick_price: TickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool {
        (working_level_type == OrderType::Buy && current_tick_price <= stop_loss_price)
            || working_level_type == OrderType::Sell && current_tick_price >= stop_loss_price
    }

    fn level_expired_by_distance(
        level_price: WLPrice,
        current_tick_price: TickPrice,
        distance_from_level_for_its_deletion: ParamValue,
    ) -> bool {
        log::debug!(
            "level_expired_by_distance: level price is {}, current tick price is {}, \
            distance from level for its deletion is {}",
            level_price,
            current_tick_price,
            distance_from_level_for_its_deletion
        );

        price_to_points((level_price - current_tick_price).abs())
            >= distance_from_level_for_its_deletion
    }

    fn level_expired_by_time(
        level_time: LevelTime,
        current_tick_time: TickTime,
        level_expiration: ParamValue,
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
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
        current_tick_price: TickPrice,
    ) -> bool {
        let level = level.as_ref();

        if (level.r#type == OrderType::Buy && current_tick_price >= level.price)
            || (level.r#type == OrderType::Sell && current_tick_price <= level.price)
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
        min_amount_of_candles_in_corridor_defining_edge_bargaining: ParamValue,
    ) -> Result<bool>
    where
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
    {
        if ParamValue::from(general_corridor.len())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::utils::entities::angle::{BasicAngleProperties, FullAngleProperties};
    use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
    use crate::step::utils::entities::working_levels::{WLId, WLMaxCrossingValue, WLStatus};
    use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
    use base::entities::candle::{BasicCandleProperties, CandleId, CandleVolatility};
    use base::entities::order::OrderId;
    use base::entities::{CandlePrices, Item};
    use base::stores::candle_store::BasicCandleStore;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    struct TestWorkingLevelStore {
        small_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
        big_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
    }

    impl TestWorkingLevelStore {
        fn default(
            small_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
            big_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
        ) -> Self {
            Self {
                small_corridor,
                big_corridor,
            }
        }
    }

    impl StepWorkingLevelStore for TestWorkingLevelStore {
        type WorkingLevelProperties = ();
        type CandleProperties = BasicCandleProperties;
        type OrderProperties = ();

        fn create_working_level(
            &mut self,
            _properties: Self::WorkingLevelProperties,
        ) -> Result<Item<WLId, Self::WorkingLevelProperties>> {
            unimplemented!()
        }

        fn get_working_level_by_id(
            &self,
            _id: &str,
        ) -> Result<Option<Item<WLId, Self::WorkingLevelProperties>>> {
            unimplemented!()
        }

        fn move_working_level_to_active(&mut self, _id: &str) -> Result<()> {
            unimplemented!()
        }

        fn remove_working_level(&mut self, _id: &str) -> Result<()> {
            unimplemented!()
        }

        fn get_created_working_levels(
            &self,
        ) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
            unimplemented!()
        }

        fn get_active_working_levels(
            &self,
        ) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
            unimplemented!()
        }

        fn get_working_level_status(&self, id: &str) -> Result<Option<WLStatus>> {
            unimplemented!()
        }

        fn clear_working_level_corridor(
            &mut self,
            working_level_id: &str,
            corridor_type: CorridorType,
        ) -> Result<()> {
            unimplemented!()
        }

        fn add_candle_to_working_level_corridor(
            &mut self,
            _working_level_id: &str,
            _candle_id: CandleId,
            _corridor_type: CorridorType,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_candles_of_working_level_corridor(
            &self,
            _working_level_id: &str,
            corridor_type: CorridorType,
        ) -> Result<Vec<Item<CandleId, Self::CandleProperties>>> {
            Ok(match corridor_type {
                CorridorType::Small => self.small_corridor.clone(),
                CorridorType::Big => self.big_corridor.clone(),
            })
        }

        fn update_max_crossing_value_of_working_level(
            &mut self,
            _working_level_id: &str,
            _default_value: WLMaxCrossingValue,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_max_crossing_value_of_working_level(
            &self,
            _working_level_id: &str,
        ) -> Result<Option<WLMaxCrossingValue>> {
            unimplemented!()
        }

        fn move_take_profits_of_level(
            &mut self,
            working_level_id: &str,
            distance_to_move_take_profits: ParamValue,
        ) -> Result<()> {
            unimplemented!()
        }

        fn take_profits_of_level_are_moved(&self, _working_level_id: &str) -> Result<bool> {
            unimplemented!()
        }

        fn add_order_to_working_level_chain_of_orders(
            &mut self,
            _working_level_id: &str,
            _order_id: OrderId,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_working_level_chain_of_orders(
            &self,
            _working_level_id: &str,
        ) -> Result<Vec<Item<OrderId, Self::OrderProperties>>> {
            unimplemented!()
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_small_corridor_is_greater_than_min_amount_of_candles__should_return_true(
    ) {
        let small_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("3"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("4"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("5"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(small_corridor, vec![]);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Small,
            dec!(3),
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_small_corridor_is_less_than_min_amount_of_candles__should_return_false(
    ) {
        let small_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(small_corridor, vec![]);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Small,
            dec!(3),
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_big_corridor_is_greater_than_min_amount_of_candles__should_return_true(
    ) {
        let big_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("3"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("4"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("5"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(vec![], big_corridor);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Big,
            dec!(3),
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_big_corridor_is_less_than_min_amount_of_candles__should_return_false(
    ) {
        let big_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(vec![], big_corridor);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Big,
            dec!(3),
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_less_than_stop_loss_price__should_return_true(
    ) {
        assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.38500),
            dec!(1.39000),
            OrderType::Buy
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_greater_than_stop_loss_price__should_return_false(
    ) {
        assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.39500),
            dec!(1.39000),
            OrderType::Buy
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_greater_than_stop_loss_price__should_return_true(
    ) {
        assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.39500),
            dec!(1.39000),
            OrderType::Sell
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_less_than_stop_loss_price__should_return_false(
    ) {
        assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.38500),
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
            dec!(1.39000),
            dec!(2_000)
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_expired_by_distance__current_tick_price_is_beyond_acceptable_range_from_level_price__should_return_true(
    ) {
        assert!(LevelConditionsImpl::level_expired_by_distance(
            dec!(1.38000),
            dec!(1.40001),
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
        let current_tick_price = dec!(1.38050);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(
            LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__have_not_returned_to_buy_level_max_crossing_value_is_beyond_limit__should_return_false(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Buy,
            ..Default::default()
        };

        let max_crossing_value = dec!(200);
        let current_tick_price = dec!(1.37999);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
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
        let current_tick_price = dec!(1.38050);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
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
        let current_tick_price = dec!(1.37999);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
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
        let current_tick_price = dec!(1.37999);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__have_not_returned_to_sell_level_max_crossing_value_is_beyond_limit__should_return_false(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Sell,
            ..Default::default()
        };

        let max_crossing_value = dec!(200);
        let current_tick_price = dec!(1.38001);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let angle = store
            .create_angle(BasicAngleProperties::default(), candle.id)
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let angle = store
            .create_angle(BasicAngleProperties::default(), candle.id)
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let min_angle = store
            .create_angle(BasicAngleProperties::default(), min_angle_candle.id.clone())
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let max_angle = store
            .create_angle(BasicAngleProperties::default(), max_angle_candle.id)
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let min_angle = store
            .create_angle(BasicAngleProperties::default(), min_angle_candle.id)
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let max_angle = store
            .create_angle(BasicAngleProperties::default(), max_angle_candle.id.clone())
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let min_angle = store
            .create_angle(BasicAngleProperties::default(), min_angle_candle.id.clone())
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let max_angle = store
            .create_angle(BasicAngleProperties::default(), max_angle_candle.id.clone())
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id.clone(),
            )
            .unwrap();

        let max_angle_before_bargaining_corridor_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle_before_bargaining_corridor = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.37000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();
        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id.clone(),
            )
            .unwrap();

        let max_angle_before_bargaining_corridor_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle_before_bargaining_corridor = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.39000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();
        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let min_angle = store
            .create_angle(BasicAngleProperties::default(), min_angle_candle.id.clone())
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let max_angle = store
            .create_angle(BasicAngleProperties::default(), max_angle_candle.id.clone())
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.37000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id.clone(),
            )
            .unwrap();

        let min_angle_before_bargaining_corridor_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.36000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle_before_bargaining_corridor = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.37000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id.clone(),
            )
            .unwrap();

        let min_angle_before_bargaining_corridor_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    leading_price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle_before_bargaining_corridor = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();
        let max_angle = store
            .create_angle(
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

        fn get_point_param_value(&self, name: Self::PointParam) -> ParamValue {
            unimplemented!()
        }

        fn get_ratio_param_value(
            &self,
            name: Self::RatioParam,
            volatility: CandleVolatility,
        ) -> ParamValue {
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();

        let min_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Max,
                    ..Default::default()
                },
                max_angle_candle.id,
            )
            .unwrap();

        let virtual_max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 6).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let virtual_max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Max,
                    ..Default::default()
                },
                max_angle_candle.id,
            )
            .unwrap();

        let virtual_max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let virtual_max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties::default())
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Max,
                    ..Default::default()
                },
                max_angle_candle.id,
            )
            .unwrap();

        let virtual_min_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 6).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let virtual_min_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Max,
                    ..Default::default()
                },
                max_angle_candle.id,
            )
            .unwrap();

        let virtual_min_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let virtual_min_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        time: NaiveDate::from_ymd(2022, 4, 4).and_hms(0, 0, 0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let min_angle = store
            .create_angle(
                BasicAngleProperties {
                    r#type: Level::Min,
                    ..Default::default()
                },
                min_angle_candle.id,
            )
            .unwrap();

        let max_angle_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let max_angle = store
            .create_angle(
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
}
