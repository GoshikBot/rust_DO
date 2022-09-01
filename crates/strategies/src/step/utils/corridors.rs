use crate::step::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::entities::working_levels::{
    BasicWLProperties, CorridorType, WLId, WLStatus,
};
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::candle::{BasicCandleProperties, CandleId};
use base::entities::order::{BasicOrderProperties, OrderType};
use base::entities::TARGET_LOGGER_ENV;
use base::entities::{CandleType, Item};
use base::helpers::{points_to_price, price_to_points};
use base::params::{ParamValue, StrategyParams};
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
        N: Fn(&C, &C, ParamValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>,

        A: Fn(&[O]) -> bool;
}

pub struct UpdateSmallCorridorNearLevelUtils<'a, C, L, N, R>
where
    C: AsRef<BasicCandleProperties> + Debug,
    L: Fn(&C) -> bool,
    N: Fn(&C, &C, ParamValue) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,
{
    pub candle_can_be_corridor_leader: &'a L,
    pub candle_is_in_corridor: &'a N,
    pub crop_corridor_to_closest_leader: &'a R,
    candle: PhantomData<C>,
}

impl<'a, C, L, N, R> UpdateSmallCorridorNearLevelUtils<'a, C, L, N, R>
where
    C: AsRef<BasicCandleProperties> + Debug,
    L: Fn(&C) -> bool,
    N: Fn(&C, &C, ParamValue) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamValue) -> bool,
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
        utils: &UpdateSmallCorridorNearLevelUtils<C, L, N, R>,
        working_level_store: &mut impl StepWorkingLevelStore<CandleProperties = C>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    ) -> Result<()>
    where
        W: AsRef<BasicWLProperties>,
        C: AsRef<BasicCandleProperties> + Debug,

        L: Fn(&C) -> bool,
        N: Fn(&C, &C, ParamValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamValue) -> bool,
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
                    target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                    "new leader of the small corridor near the level: level — ({:?}), leader — {:?}",
                    level, current_candle.props.as_ref()
                );

                working_level_store.add_candle_to_working_level_corridor(
                    level.id,
                    current_candle.id.clone(),
                    CorridorType::Small,
                )?;
            }
        } else if (utils.candle_is_in_corridor)(
            &current_candle.props,
            &corridor_candles[0].props,
            params
                .get_point_param_value(StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct),
        ) {
            log::debug!(
                target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                "new candle of the small corridor near the level: level — ({:?}), new_candle — {:?}, \
                corridor_candles — {:?}", level, current_candle.props.as_ref(),
                corridor_candles
            );

            working_level_store.add_candle_to_working_level_corridor(
                level.id,
                current_candle.id.clone(),
                CorridorType::Small,
            )?;
        } else if (dbg!(distance_from_candle_to_level)
            <= dbg!(distance_from_level_to_corridor_before_activation_crossing_of_level)
            && ParamValue::from(corridor_candles.len()) < min_amount_of_candles_in_small_corridor)
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

            match new_corridor {
                Some(corridor_candles) => {
                    for candle in corridor_candles {
                        working_level_store.add_candle_to_working_level_corridor(
                            level.id,
                            candle.id.clone(),
                            CorridorType::Small,
                        )?;
                    }
                }
                None => {
                    if candle_can_be_corridor_leader(&current_candle.props) {
                        working_level_store.add_candle_to_working_level_corridor(
                            level.id,
                            current_candle.id.clone(),
                            CorridorType::Small,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    fn update_big_corridor_near_level<W, C>(
        level: &Item<WLId, W>,
        current_candle: &Item<CandleId, C>,
        working_level_store: &mut impl StepWorkingLevelStore<CandleProperties = C>,
        range_of_big_corridor_near_level: ParamValue,
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
                target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
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
                target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
                "current candle is out of the range of the big corridor near the level: \
                level — {:?}, current_candle — {:?}", level, current_candle
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
    N: Fn(&C, &C, ParamValue) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,

    A: Fn(&[O]) -> bool,
{
    pub small_corridor_utils: UpdateSmallCorridorNearLevelUtils<'a, C, L, N, R>,
    pub level_has_no_active_orders: &'a A,
    order: PhantomData<O>,
}

impl<'a, C, O, L, N, R, A> UpdateCorridorsNearWorkingLevelsUtils<'a, C, O, L, N, R, A>
where
    C: AsRef<BasicCandleProperties> + Debug,
    O: AsRef<BasicOrderProperties>,

    N: Fn(&C, &C, ParamValue) -> bool,
    L: Fn(&C) -> bool,
    R: Fn(
        &[Item<CandleId, C>],
        &Item<CandleId, C>,
        ParamValue,
        &dyn Fn(&C) -> bool,
        &dyn Fn(&C, &C, ParamValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>,

    A: Fn(&[O]) -> bool,
{
    pub fn new(
        small_corridor_utils: UpdateSmallCorridorNearLevelUtils<'a, C, L, N, R>,
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
        N: Fn(&C, &C, ParamValue) -> bool,
        R: Fn(
            &[Item<CandleId, C>],
            &Item<CandleId, C>,
            ParamValue,
            &dyn Fn(&C) -> bool,
            &dyn Fn(&C, &C, ParamValue) -> bool,
        ) -> Option<Vec<Item<CandleId, C>>>,

        A: Fn(&[O]) -> bool,
    {
        for level in working_level_store
            .get_created_working_levels()?
            .into_iter()
            .chain(working_level_store.get_active_working_levels()?.into_iter())
            .map(|level| Item {
                id: level.id,
                props: level.props.into(),
            })
        {
            if working_level_store
                .get_working_level_status(&level.id)?
                .unwrap()
                == WLStatus::Active
                && !(utils.level_has_no_active_orders)(
                    &working_level_store
                        .get_working_level_chain_of_orders(&level.id)?
                        .into_iter()
                        .map(|order| order.props)
                        .collect::<Vec<_>>(),
                )
            {
                continue;
            }

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
}

#[cfg(test)]
mod tests {
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

        fn get_point_param_value(&self, name: Self::PointParam) -> ParamValue {
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
        ) -> ParamValue {
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
    // - 10. active working level with active orders
    // - 11. active working level without active orders
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
    //

    #[test]
    #[allow(non_snake_case)]
    fn update_corridors_near_working_levels__small_corridor_is_empty_and_candle_can_be_corridor_leader_and_buy_level_and_green_candle_and_candle_is_in_the_range_of_big_corridor__should_add_candle_to_small_and_big_corridors(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();
        let working_level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;
        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;
        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let corridor_candle = store.create_candle(Default::default()).unwrap();
        store
            .add_candle_to_working_level_corridor(
                &working_level.id,
                corridor_candle.id,
                CorridorType::Small,
            )
            .unwrap();

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| true;
        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..3 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;
        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..2 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;
        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..2 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
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
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;
        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..2 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
            store
                .add_candle_to_working_level_corridor(
                    &working_level.id,
                    corridor_candle.id,
                    CorridorType::Small,
                )
                .unwrap();
        }

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;

        let mut new_corridor = Vec::new();

        for _ in 0..3 {
            let candle = store.create_candle(Default::default()).unwrap();
            new_corridor.push(candle);
        }

        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| Some(new_corridor.clone());

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..2 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
            store
                .add_candle_to_working_level_corridor(
                    &working_level.id,
                    corridor_candle.id,
                    CorridorType::Small,
                )
                .unwrap();
        }

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;

        let mut new_corridor = Vec::new();

        for _ in 0..3 {
            let candle = store.create_candle(Default::default()).unwrap();
            new_corridor.push(candle);
        }

        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| Some(new_corridor.clone());

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..2 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
            store
                .add_candle_to_working_level_corridor(
                    &working_level.id,
                    corridor_candle.id,
                    CorridorType::Small,
                )
                .unwrap();
        }

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;

        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
    fn update_corridors_near_working_levels__inactive_level_and_level_with_active_orders_sell_level_and_green_candle_and_candle_is_not_in_the_range_of_big_corridor__should_not_update_corridors_of_active_level_with_active_orders_and_should_clear_big_corridor(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();
        let inactive_working_level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let active_working_level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        store
            .move_working_level_to_active(&active_working_level.id)
            .unwrap();

        let mut big_corridor = Vec::new();

        for _ in 0..3 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();
            big_corridor.push(corridor_candle.clone());

            store
                .add_candle_to_working_level_corridor(
                    &inactive_working_level.id,
                    corridor_candle.id.clone(),
                    CorridorType::Big,
                )
                .unwrap();

            store
                .add_candle_to_working_level_corridor(
                    &active_working_level.id,
                    corridor_candle.id,
                    CorridorType::Big,
                )
                .unwrap();
        }

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Green,
                        prices: CandlePrices {
                            open: dec!(1.37799),
                            high: dec!(1.37971),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;

        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| false;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
                    &candle_can_be_corridor_leader,
                    &candle_is_in_corridor,
                    &crop_corridor_to_the_closest_leader,
                ),
                &level_has_no_active_orders,
            ),
            &params,
        )
        .unwrap();

        let small_corridor_of_inactive_level = store
            .get_candles_of_working_level_corridor(&inactive_working_level.id, CorridorType::Small)
            .unwrap();

        let small_corridor_of_active_level = store
            .get_candles_of_working_level_corridor(&active_working_level.id, CorridorType::Small)
            .unwrap();

        assert!(
            small_corridor_of_inactive_level.len() == 1
                && small_corridor_of_inactive_level.contains(&current_candle)
        );
        assert!(small_corridor_of_active_level.is_empty());

        let big_corridor_of_inactive_level = store
            .get_candles_of_working_level_corridor(&inactive_working_level.id, CorridorType::Big)
            .unwrap();

        let big_corridor_of_active_level = store
            .get_candles_of_working_level_corridor(&active_working_level.id, CorridorType::Big)
            .unwrap();

        assert!(big_corridor_of_inactive_level.is_empty());
        assert_eq!(big_corridor_of_active_level, big_corridor);
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_corridors_near_working_levels__inactive_level_and_level_without_active_orders_sell_level_and_neutral_candle_and_candle_is_not_in_the_range_of_big_corridor__should_update_corridors_of_active_level_without_active_orders_and_should_clear_big_corridor(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();
        let inactive_working_level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let active_working_level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        store
            .move_working_level_to_active(&active_working_level.id)
            .unwrap();

        for _ in 0..3 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();

            store
                .add_candle_to_working_level_corridor(
                    &inactive_working_level.id,
                    corridor_candle.id.clone(),
                    CorridorType::Big,
                )
                .unwrap();

            store
                .add_candle_to_working_level_corridor(
                    &active_working_level.id,
                    corridor_candle.id,
                    CorridorType::Big,
                )
                .unwrap();
        }

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
                step_common: StepCandleProperties {
                    base: BasicCandleProperties {
                        r#type: CandleType::Neutral,
                        prices: CandlePrices {
                            open: dec!(1.37799),
                            high: dec!(1.37971),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| true;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;

        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
                    &candle_can_be_corridor_leader,
                    &candle_is_in_corridor,
                    &crop_corridor_to_the_closest_leader,
                ),
                &level_has_no_active_orders,
            ),
            &params,
        )
        .unwrap();

        let small_corridor_of_inactive_level = store
            .get_candles_of_working_level_corridor(&inactive_working_level.id, CorridorType::Small)
            .unwrap();

        let small_corridor_of_active_level = store
            .get_candles_of_working_level_corridor(&active_working_level.id, CorridorType::Small)
            .unwrap();

        assert!(
            small_corridor_of_inactive_level.len() == 1
                && small_corridor_of_inactive_level.contains(&current_candle)
        );
        assert!(
            small_corridor_of_active_level.len() == 1
                && small_corridor_of_active_level.contains(&current_candle)
        );

        let big_corridor_of_inactive_level = store
            .get_candles_of_working_level_corridor(&inactive_working_level.id, CorridorType::Big)
            .unwrap();

        let big_corridor_of_active_level = store
            .get_candles_of_working_level_corridor(&active_working_level.id, CorridorType::Big)
            .unwrap();

        assert!(big_corridor_of_inactive_level.is_empty());
        assert!(big_corridor_of_active_level.is_empty());
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_corridors_near_working_levels__sell_level_and_red_candle_and_candle_is_not_in_the_range_of_big_corridor__should_clear_big_corridor(
    ) {
        let mut store = InMemoryStepBacktestingStore::new();
        let working_level = store
            .create_working_level(BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();

        for _ in 0..3 {
            let corridor_candle = store.create_candle(Default::default()).unwrap();

            store
                .add_candle_to_working_level_corridor(
                    &working_level.id,
                    corridor_candle.id.clone(),
                    CorridorType::Big,
                )
                .unwrap();
        }

        let current_candle = store
            .create_candle(StepBacktestingCandleProperties {
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
            })
            .unwrap();

        let candle_can_be_corridor_leader = |_: &StepBacktestingCandleProperties| false;
        let candle_is_in_corridor = |_: &StepBacktestingCandleProperties,
                                     _: &StepBacktestingCandleProperties,
                                     _: ParamValue| false;

        let crop_corridor_to_the_closest_leader =
            |_: &[Item<CandleId, StepBacktestingCandleProperties>],
             _: &Item<CandleId, StepBacktestingCandleProperties>,
             _: ParamValue,
             _: &dyn Fn(&StepBacktestingCandleProperties) -> bool,
             _: &dyn Fn(
                &StepBacktestingCandleProperties,
                &StepBacktestingCandleProperties,
                ParamValue,
            ) -> bool| None;

        let level_has_no_active_orders = |_: &[StepOrderProperties]| true;

        let params = TestParams::default();

        CorridorsImpl::update_corridors_near_working_levels(
            &mut store,
            &current_candle,
            UpdateCorridorsNearWorkingLevelsUtils::new(
                UpdateSmallCorridorNearLevelUtils::new(
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
}
