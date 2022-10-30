use crate::step::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::entities::working_levels::{
    BasicWLProperties, CorridorType, WLId, WLStatus,
};
use crate::step::utils::stores::candle_store::StepCandleStore;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::candle::{BasicCandleProperties, CandleId};
use base::entities::order::{BasicOrderProperties, OrderType};
use base::entities::{CandleType, Item};
use base::helpers::{points_to_price, price_to_points};
use base::params::{ParamOutputValue, StrategyParams};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait Corridors {
    /// Accumulates candles near the working levels before activation crossing of these levels.
    /// When the definite corridor appears, it's the signal to remove such working level as invalid.
    fn update_corridors_near_working_levels<W, O, C, L, N, R, A>(
        working_level_store: &mut impl StepWorkingLevelStore<
            WorkingLevelProperties = W,
            OrderProperties = O,
            CandleProperties = C,
        >,
        current_candle: &Item<CandleId, C>,
        utils: UpdateCorridorsNearWorkingLevelsUtils<C, O, L, N, R, A>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<()>
    where
        W: Into<BasicWLProperties>,
        O: AsRef<BasicOrderProperties>,

        C: AsRef<BasicCandleProperties> + Debug,
        L: Fn(&C) -> bool,
        N: Fn(&C, &C, ParamOutputValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamOutputValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamOutputValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>,

        A: Fn(&[O]) -> bool;

    /// Accumulates candles into the general corridor and clears the corridor
    /// when the new candle goes beyond the defined range.
    fn update_general_corridor<C, L, N, R>(
        current_candle: &Item<CandleId, C>,
        candle_store: &mut impl StepCandleStore<CandleProperties = C>,
        utils: UpdateGeneralCorridorUtils<C, L, N, R>,
        max_distance_from_corridor_leading_candle_pins_pct: ParamOutputValue,
    ) -> Result<()>
    where
        C: AsRef<BasicCandleProperties> + Debug,
        L: Fn(&C) -> bool,
        N: Fn(&C, &C, ParamOutputValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamOutputValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamOutputValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>;
}

pub struct UpdateGeneralCorridorUtils<'a, C, L, N, R>
where
    C: AsRef<BasicCandleProperties> + Debug,
    L: Fn(&C) -> bool,
    N: Fn(&C, &C, ParamOutputValue) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamOutputValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamOutputValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,
{
    pub candle_can_be_corridor_leader: &'a L,
    pub candle_is_in_corridor: &'a N,
    pub crop_corridor_to_closest_leader: &'a R,
    candle: PhantomData<C>,
}

impl<'a, C, L, N, R> UpdateGeneralCorridorUtils<'a, C, L, N, R>
where
    C: AsRef<BasicCandleProperties> + Debug,
    L: Fn(&C) -> bool,
    N: Fn(&C, &C, ParamOutputValue) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamOutputValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamOutputValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,
{
    pub fn new(
        candle_can_be_corridor_leader: &'a L,
        candle_is_in_corridor: &'a N,
        crop_corridor_to_closest_leader: &'a R,
    ) -> Self {
        Self {
            candle_can_be_corridor_leader,
            candle_is_in_corridor,
            crop_corridor_to_closest_leader,
            candle: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct CorridorsImpl;

impl CorridorsImpl {
    pub fn new() -> Self {
        Self::default()
    }

    fn update_small_corridor_near_level<W, C, L, N, R>(
        level: &Item<WLId, W>,
        current_candle: &Item<CandleId, C>,
        utils: &UpdateGeneralCorridorUtils<C, L, N, R>,
        working_level_store: &mut impl StepWorkingLevelStore<CandleProperties = C>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<()>
    where
        W: AsRef<BasicWLProperties>,
        C: AsRef<BasicCandleProperties> + Debug,

        L: Fn(&C) -> bool,
        N: Fn(&C, &C, ParamOutputValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamOutputValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamOutputValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>,
    {
        let level = Item {
            id: &level.id,
            props: level.props.as_ref(),
        };

        let min_amount_of_candles_in_small_corridor = params.get_point_param_value(
            StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel,
        );

        let distance_from_candle_to_level = price_to_points(match level.props.r#type {
            OrderType::Buy => current_candle.props.as_ref().prices.low - level.props.price,
            OrderType::Sell => level.props.price - current_candle.props.as_ref().prices.high,
        });

        let distance_from_level_to_corridor_before_activation_crossing_of_level = params
            .get_ratio_param_value(
                StepRatioParam::DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel,
                current_candle.props.as_ref().volatility,
            );

        let candle_can_be_corridor_leader = |candle: &C| {
            (utils.candle_can_be_corridor_leader)(candle)
                && distance_from_candle_to_level
                    <= distance_from_level_to_corridor_before_activation_crossing_of_level
        };

        let corridor_candles = working_level_store
            .get_candles_of_working_level_corridor(level.id, CorridorType::Small)?;

        if corridor_candles.is_empty() {
            if candle_can_be_corridor_leader(&current_candle.props) {
                log::debug!(
                    "new leader of the small corridor near the level: level — ({:?}), leader — {:?}",
                    level, current_candle.props.as_ref()
                );

                working_level_store.add_candle_to_working_level_corridor(
                    level.id,
                    current_candle.id.clone(),
                    CorridorType::Small,
                )?;
            } else {
                log::debug!(
                    "the new candle cannot be the corridor leader: distance from candle \
                    to level — {distance_from_candle_to_level}, distance from level to corridor \
                    before activation crossing of level — {distance_from_level_to_corridor_before_activation_crossing_of_level}, \
                    current candle — {current_candle:?}",
                );
            }
        } else if (utils.candle_is_in_corridor)(
            &current_candle.props,
            &corridor_candles[0].props,
            params
                .get_point_param_value(StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct),
        ) {
            log::debug!(
                "new candle of the small corridor near the level: level — ({:?}), new_candle — {:?}, \
                corridor_candles — {:?}", level, current_candle.props.as_ref(),
                corridor_candles
            );

            working_level_store.add_candle_to_working_level_corridor(
                level.id,
                current_candle.id.clone(),
                CorridorType::Small,
            )?;
        } else if (distance_from_candle_to_level
            <= distance_from_level_to_corridor_before_activation_crossing_of_level
            && ParamOutputValue::from(corridor_candles.len())
                < min_amount_of_candles_in_small_corridor)
            || distance_from_candle_to_level
                > distance_from_level_to_corridor_before_activation_crossing_of_level
        {
            let new_corridor = (utils.crop_corridor_to_closest_leader)(
                &corridor_candles,
                current_candle,
                params.get_point_param_value(
                    StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct,
                ),
                &candle_can_be_corridor_leader,
                utils.candle_is_in_corridor,
            );

            working_level_store.clear_working_level_corridor(level.id, CorridorType::Small)?;

            log::debug!("clear small corridor near the level: level — {level:?}");

            match new_corridor {
                Some(corridor_candles) => {
                    log::debug!("new cropped small corridor: {:?}", corridor_candles);

                    for candle in corridor_candles {
                        working_level_store.add_candle_to_working_level_corridor(
                            level.id,
                            candle.id.clone(),
                            CorridorType::Small,
                        )?;
                    }
                }
                None => {
                    log::debug!("new cropped small corridor is empty");

                    if candle_can_be_corridor_leader(&current_candle.props) {
                        working_level_store.add_candle_to_working_level_corridor(
                            level.id,
                            current_candle.id.clone(),
                            CorridorType::Small,
                        )?;

                        log::debug!(
                            "new leader of the small corridor near the level: level — ({:?}), leader — {:?}",
                            level, current_candle
                        );
                    }
                }
            }
        } else {
            log::debug!(
                "inappropriate conditions to check the case when the new candle is beyond the corridor: \
                distance from candle to level — {}, distance from level to corridor before \
                activation crossing of level — {}, len of corridor — {}, min amount of candles in \
                small corridor — {}",
                distance_from_candle_to_level,
                distance_from_level_to_corridor_before_activation_crossing_of_level,
                corridor_candles.len(),
                min_amount_of_candles_in_small_corridor
            );
        }

        Ok(())
    }

    fn update_big_corridor_near_level<W, C>(
        level: &Item<WLId, W>,
        current_candle: &Item<CandleId, C>,
        working_level_store: &mut impl StepWorkingLevelStore<CandleProperties = C>,
        range_of_big_corridor_near_level: ParamOutputValue,
    ) -> Result<()>
    where
        W: AsRef<BasicWLProperties> + Debug,
        C: AsRef<BasicCandleProperties> + Debug,
    {
        let range_of_big_corridor_near_level = points_to_price(range_of_big_corridor_near_level);

        let (edge_of_corridor_range, current_candle_edge_price, orderings) =
            match level.props.as_ref().r#type {
                OrderType::Buy => {
                    let edge_of_corridor_range =
                        level.props.as_ref().price + range_of_big_corridor_near_level;

                    let candle_edge_price = match current_candle.props.as_ref().r#type {
                        CandleType::Green | CandleType::Neutral => {
                            current_candle.props.as_ref().prices.close
                        }
                        CandleType::Red => current_candle.props.as_ref().prices.open,
                    };

                    (
                        edge_of_corridor_range,
                        candle_edge_price,
                        [Ordering::Less, Ordering::Equal],
                    )
                }
                OrderType::Sell => {
                    let edge_of_corridor_range =
                        level.props.as_ref().price - range_of_big_corridor_near_level;

                    let candle_edge_price = match current_candle.props.as_ref().r#type {
                        CandleType::Green | CandleType::Neutral => {
                            current_candle.props.as_ref().prices.open
                        }
                        CandleType::Red => current_candle.props.as_ref().prices.close,
                    };

                    (
                        edge_of_corridor_range,
                        candle_edge_price,
                        [Ordering::Greater, Ordering::Equal],
                    )
                }
            };

        if orderings.contains(&current_candle_edge_price.cmp(&edge_of_corridor_range)) {
            log::debug!(
                "new candle of the big corridor near the level: level — {:?}, current_candle — {:?}",
                level, current_candle
            );

            working_level_store.add_candle_to_working_level_corridor(
                &level.id,
                current_candle.id.clone(),
                CorridorType::Big,
            )?;
        } else {
            log::debug!(
                "current candle is out of the range of the big corridor near the level: \
                level — {:?}, current_candle — {:?}",
                level,
                current_candle
            );

            working_level_store.clear_working_level_corridor(&level.id, CorridorType::Big)?;
        }

        Ok(())
    }
}

pub struct UpdateCorridorsNearWorkingLevelsUtils<'a, C, O, L, N, R, A>
where
    C: AsRef<BasicCandleProperties> + Debug,
    O: AsRef<BasicOrderProperties>,

    L: Fn(&C) -> bool,
    N: Fn(&C, &C, ParamOutputValue) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamOutputValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamOutputValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,

    A: Fn(&[O]) -> bool,
{
    pub small_corridor_utils: UpdateGeneralCorridorUtils<'a, C, L, N, R>,
    pub level_has_no_active_orders: &'a A,
    order: PhantomData<O>,
}

impl<'a, C, O, L, N, R, A> UpdateCorridorsNearWorkingLevelsUtils<'a, C, O, L, N, R, A>
where
    C: AsRef<BasicCandleProperties> + Debug,
    O: AsRef<BasicOrderProperties>,

    N: Fn(&C, &C, ParamOutputValue) -> bool,
    L: Fn(&C) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamOutputValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamOutputValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,

    A: Fn(&[O]) -> bool,
{
    pub fn new(
        small_corridor_utils: UpdateGeneralCorridorUtils<'a, C, L, N, R>,
        level_has_no_active_orders: &'a A,
    ) -> Self {
        Self {
            small_corridor_utils,
            level_has_no_active_orders,
            order: PhantomData,
        }
    }
}

impl Corridors for CorridorsImpl {
    fn update_corridors_near_working_levels<W, O, C, L, N, R, A>(
        working_level_store: &mut impl StepWorkingLevelStore<
            WorkingLevelProperties = W,
            OrderProperties = O,
            CandleProperties = C,
        >,
        current_candle: &Item<CandleId, C>,
        utils: UpdateCorridorsNearWorkingLevelsUtils<C, O, L, N, R, A>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<()>
    where
        W: Into<BasicWLProperties>,
        O: AsRef<BasicOrderProperties>,

        C: AsRef<BasicCandleProperties> + Debug,
        L: Fn(&C) -> bool,
        N: Fn(&C, &C, ParamOutputValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamOutputValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamOutputValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>,

        A: Fn(&[O]) -> bool,
    {
        for level in working_level_store
            .get_created_working_levels()?
            .into_iter()
            .map(|level| Item {
                id: level.id,
                props: level.props.into(),
            })
        {
            Self::update_small_corridor_near_level(
                &level,
                current_candle,
                &utils.small_corridor_utils,
                working_level_store,
                params,
            )?;

            Self::update_big_corridor_near_level(
                &level,
                current_candle,
                working_level_store,
                params.get_ratio_param_value(
                    StepRatioParam::RangeOfBigCorridorNearLevel,
                    current_candle.props.as_ref().volatility,
                ),
            )?;
        }

        Ok(())
    }

    fn update_general_corridor<C, L, N, R>(
        current_candle: &Item<CandleId, C>,
        candle_store: &mut impl StepCandleStore<CandleProperties = C>,
        utils: UpdateGeneralCorridorUtils<C, L, N, R>,
        max_distance_from_corridor_leading_candle_pins_pct: ParamOutputValue,
    ) -> Result<()>
    where
        C: AsRef<BasicCandleProperties> + Debug,
        L: Fn(&C) -> bool,
        N: Fn(&C, &C, ParamOutputValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamOutputValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamOutputValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>,
    {
        let corridor_candles = candle_store.get_candles_of_general_corridor()?;

        if corridor_candles.is_empty() {
            if (utils.candle_can_be_corridor_leader)(&current_candle.props) {
                log::debug!(
                    "new general corridor leader: {:?}",
                    current_candle.props.as_ref()
                );

                candle_store.add_candle_to_general_corridor(current_candle.id.clone())?;
            } else {
                log::debug!(
                    "candle can't be general corridor leader: {:?}",
                    current_candle
                );
            }
        } else if (utils.candle_is_in_corridor)(
            &current_candle.props,
            &corridor_candles[0].props,
            max_distance_from_corridor_leading_candle_pins_pct,
        ) {
            log::debug!(
                "new candle of the general corridor: new_candle — {:?}, \
                corridor_candles — {:?}",
                current_candle.props.as_ref(),
                corridor_candles
            );

            candle_store.add_candle_to_general_corridor(current_candle.id.clone())?;
        } else {
            let new_corridor = (utils.crop_corridor_to_closest_leader)(
                &corridor_candles,
                current_candle,
                max_distance_from_corridor_leading_candle_pins_pct,
                utils.candle_can_be_corridor_leader,
                utils.candle_is_in_corridor,
            );

            candle_store.clear_general_corridor()?;

            log::debug!("clear general corridor");

            match new_corridor {
                Some(corridor_candles) => {
                    log::debug!("new cropped general corridor: {:?}", corridor_candles);

                    for candle in corridor_candles {
                        candle_store.add_candle_to_general_corridor(candle.id)?;
                    }
                }
                None => {
                    log::debug!("new cropped general corridor is empty");

                    if (utils.candle_can_be_corridor_leader)(&current_candle.props) {
                        log::debug!("new general corridor leader: {:?}", current_candle);

                        candle_store.add_candle_to_general_corridor(current_candle.id.clone())?;
                    } else {
                        log::debug!(
                            "candle can't be general corridor leader: {:?}",
                            current_candle
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
