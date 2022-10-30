use crate::step::utils::backtesting_charts::{
    ChartIndex, ChartTraceEntity, StepBacktestingChartTraces,
};
use crate::step::utils::entities::angle::{AngleId, BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::{StepBacktestingCandleProperties, StepCandleProperties};
use crate::step::utils::entities::order::StepOrderProperties;
use crate::step::utils::entities::params::{StepPointParam, StepRatioParam};
use crate::step::utils::entities::working_levels::{
    LevelTime, WLMaxCrossingValue, WLPrice, WLStatus,
};
use crate::step::utils::entities::{Mode, StatisticsChartsNotifier, StatisticsNotifier, MODE_ENV};
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::stores::angle_store::StepAngleStore;
use crate::step::utils::stores::candle_store::StepCandleStore;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use crate::step::utils::stores::StepConfig;
use anyhow::{Context, Result};
use base::entities::candle::{CandleId, CandleVolatility};
use base::entities::order::{BasicOrderProperties, OrderStatus, OrderType};
use base::entities::tick::{TickPrice, TickTime, UniversalTickPrice};
use base::entities::{BasicTickProperties, Item, Level, Tendency};
use base::helpers::{price_to_points, Holiday, NumberOfDaysToExclude};
use base::notifier::NotificationQueue;
use base::params::{ParamOutputValue, StrategyParams};
use chrono::NaiveDateTime;
use rust_decimal_macros::dec;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::FromStr;

use super::entities::working_levels::{BasicWLProperties, WLId};

pub trait LevelUtils {
    /// Checks whether one of the working levels has got crossed and returns such a level.
    fn get_crossed_level<W>(
        current_tick_price: UniversalTickPrice,
        created_working_levels: &[Item<WLId, W>],
    ) -> Option<&Item<WLId, W>>
    where
        W: AsRef<BasicWLProperties>;

    /// Moves active working levels to removed if they have closed orders in their chains.
    fn remove_active_working_levels_with_closed_orders<O>(
        working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
    ) -> Result<()>
    where
        O: Into<StepOrderProperties>;

    /// Updates the activation max crossing distance for active levels.
    /// It's required to delete invalid active levels that crossed particular distance
    /// and returned to level without getting to the first order.
    fn update_max_crossing_value_of_working_levels<T>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
        current_tick_price: UniversalTickPrice,
    ) -> Result<()>
    where
        T: Into<BasicWLProperties>;

    fn remove_invalid_working_levels<W, A, D, M, C, E, T, N, O>(
        current_tick: &BasicTickProperties<UniversalTickPrice>,
        current_volatility: CandleVolatility,
        utils: RemoveInvalidWorkingLevelsUtils<W, A, D, M, C, E, T, O>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        entity: StatisticsNotifier<N>,
    ) -> Result<()>
    where
        T: AsRef<BasicWLProperties>,
        O: AsRef<BasicOrderProperties>,
        W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
        A: Fn(&[O]) -> bool,
        D: Fn(WLPrice, UniversalTickPrice, ParamOutputValue) -> bool,
        M: Fn(LevelTime, TickTime, ParamOutputValue, &E) -> bool,
        C: Fn(&T, Option<WLMaxCrossingValue>, ParamOutputValue, UniversalTickPrice) -> bool,
        E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
        N: NotificationQueue;

    /// Moves take profits of the existing chains of orders when the current tick price
    /// deviates from the active working level on the defined amount of points.
    fn move_take_profits<W>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        distance_from_level_for_signaling_of_moving_take_profits: ParamOutputValue,
        distance_to_move_take_profits: ParamOutputValue,
        current_tick_price: UniversalTickPrice,
    ) -> Result<()>
    where
        W: Into<BasicWLProperties>;

    fn update_tendency_and_get_instruction_to_create_new_working_level<
        S,
        D,
        A,
        C,
        N,
        H,
        B,
        P,
        M,
        K,
        X,
        L,
    >(
        config: &mut StepConfig,
        store: &mut S,
        utils: UpdateTendencyAndCreateWorkingLevelUtils<D, A, C, S, B, P, M, K, X, L>,
        statistics_charts_notifier: StatisticsChartsNotifier<N, H>,
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        current_candle: &Item<CandleId, C>,
        params: &M,
    ) -> Result<bool>
    where
        S: StepAngleStore<AngleProperties = A, CandleProperties = C>
            + StepCandleStore<CandleProperties = C>
            + StepWorkingLevelStore<WorkingLevelProperties = K>,
        D: Fn(&str, Option<&str>, bool, bool) -> bool,
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
        N: NotificationQueue,
        H: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
        B: Fn(
            &Item<AngleId, FullAngleProperties<A, C>>,
            &[Item<CandleId, C>],
            &S,
            ParamOutputValue,
        ) -> Result<bool>,
        M: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        P: Fn(
            &Item<AngleId, FullAngleProperties<A, C>>,
            &Item<CandleId, C>,
            &S,
            &M,
        ) -> Result<bool>,
        K: AsRef<BasicWLProperties>,
        X: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S) -> Result<bool>,
        L: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S, ParamOutputValue) -> Result<bool>;
}

pub struct RemoveInvalidWorkingLevelsUtils<'a, W, A, D, M, C, E, T, O>
where
    T: AsRef<BasicWLProperties>,
    O: AsRef<BasicOrderProperties>,
    W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
    A: Fn(&[O]) -> bool,
    D: Fn(WLPrice, UniversalTickPrice, ParamOutputValue) -> bool,
    M: Fn(LevelTime, TickTime, ParamOutputValue, &E) -> bool,
    C: Fn(&T, Option<WLMaxCrossingValue>, ParamOutputValue, UniversalTickPrice) -> bool,
    E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
{
    pub working_level_store: &'a mut W,
    pub level_has_no_active_orders: &'a A,
    pub level_expired_by_distance: &'a D,
    pub level_expired_by_time: &'a M,
    pub active_level_exceeds_activation_crossing_distance_when_returned_to_level: &'a C,
    pub exclude_weekend_and_holidays: &'a E,
}

pub struct UpdateTendencyAndCreateWorkingLevelUtils<'a, D, A, C, S, B, P, M, K, X, L>
where
    D: Fn(&str, Option<&str>, bool, bool) -> bool,
    A: AsRef<BasicAngleProperties> + Debug,
    C: AsRef<StepCandleProperties> + Debug + PartialEq,
    S: StepAngleStore<AngleProperties = A, CandleProperties = C>
        + StepWorkingLevelStore<WorkingLevelProperties = K>,
    B: Fn(
        &Item<AngleId, FullAngleProperties<A, C>>,
        &[Item<CandleId, C>],
        &S,
        ParamOutputValue,
    ) -> Result<bool>,
    M: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    P: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &Item<CandleId, C>, &S, &M) -> Result<bool>,
    K: AsRef<BasicWLProperties>,
    X: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S) -> Result<bool>,
    L: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S, ParamOutputValue) -> Result<bool>,
{
    pub is_second_level_after_bargaining_tendency_change: &'a D,
    pub level_comes_out_of_bargaining_corridor: &'a B,
    pub appropriate_working_level: &'a P,
    pub working_level_exists: &'a X,
    pub working_level_is_close_to_another_one: &'a L,
    angle: PhantomData<A>,
    candle: PhantomData<C>,
    store: PhantomData<S>,
    working_level: PhantomData<K>,
    params: PhantomData<M>,
}

impl<'a, D, A, C, S, B, P, M, K, X, L>
    UpdateTendencyAndCreateWorkingLevelUtils<'a, D, A, C, S, B, P, M, K, X, L>
where
    D: Fn(&str, Option<&str>, bool, bool) -> bool,
    A: AsRef<BasicAngleProperties> + Debug,
    C: AsRef<StepCandleProperties> + Debug + PartialEq,
    S: StepAngleStore<AngleProperties = A, CandleProperties = C>
        + StepWorkingLevelStore<WorkingLevelProperties = K>,
    B: Fn(
        &Item<AngleId, FullAngleProperties<A, C>>,
        &[Item<CandleId, C>],
        &S,
        ParamOutputValue,
    ) -> Result<bool>,
    M: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    P: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &Item<CandleId, C>, &S, &M) -> Result<bool>,
    K: AsRef<BasicWLProperties>,
    X: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S) -> Result<bool>,
    L: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S, ParamOutputValue) -> Result<bool>,
{
    pub fn new(
        is_second_level_after_bargaining_tendency_change: &'a D,
        level_comes_out_of_bargaining_corridor: &'a B,
        appropriate_working_level: &'a P,
        working_level_exists: &'a X,
        working_level_is_close_to_another_one: &'a L,
    ) -> Self {
        Self {
            is_second_level_after_bargaining_tendency_change,
            level_comes_out_of_bargaining_corridor,
            appropriate_working_level,
            working_level_exists,
            working_level_is_close_to_another_one,
            angle: PhantomData,
            candle: PhantomData,
            store: PhantomData,
            params: PhantomData,
            working_level: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct LevelUtilsImpl;

impl LevelUtilsImpl {
    pub fn new() -> Self {
        Self::default()
    }

    fn working_level_has_closed_orders_in_chain(chain_of_orders: &[StepOrderProperties]) -> bool {
        chain_of_orders
            .iter()
            .any(|order| order.base.status == OrderStatus::Closed)
    }
}

impl LevelUtils for LevelUtilsImpl {
    fn get_crossed_level<W>(
        current_tick_price: UniversalTickPrice,
        created_working_levels: &[Item<WLId, W>],
    ) -> Option<&Item<WLId, W>>
    where
        W: AsRef<BasicWLProperties>,
    {
        let (lowest_current_tick_price, highest_current_tick_price) = match current_tick_price {
            UniversalTickPrice::Historical(current_tick_price) => {
                (current_tick_price.low, current_tick_price.high)
            }
            UniversalTickPrice::Realtime(current_tick_price) => {
                (current_tick_price, current_tick_price)
            }
        };

        for level in created_working_levels {
            let level_properties = level.props.as_ref();

            match level_properties.r#type {
                OrderType::Buy => {
                    if lowest_current_tick_price < level_properties.price {
                        return Some(level);
                    }
                }
                OrderType::Sell => {
                    if highest_current_tick_price > level_properties.price {
                        return Some(level);
                    }
                }
            }
        }

        None
    }

    fn remove_active_working_levels_with_closed_orders<O>(
        working_level_store: &mut impl StepWorkingLevelStore<OrderProperties = O>,
    ) -> Result<()>
    where
        O: Into<StepOrderProperties>,
    {
        for level in working_level_store.get_active_working_levels()? {
            let level_chain_of_orders: Vec<_> = working_level_store
                .get_working_level_chain_of_orders(&level.id)?
                .into_iter()
                .map(|order| order.props.into())
                .collect();

            if Self::working_level_has_closed_orders_in_chain(&level_chain_of_orders) {
                working_level_store.remove_working_level(&level.id)?;
            }
        }

        Ok(())
    }

    fn update_max_crossing_value_of_working_levels<T>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = T>,
        current_tick_price: UniversalTickPrice,
    ) -> Result<()>
    where
        T: Into<BasicWLProperties>,
    {
        let (lowest_current_tick_price, highest_current_tick_price) = match current_tick_price {
            UniversalTickPrice::Historical(current_tick_price) => {
                (current_tick_price.low, current_tick_price.high)
            }
            UniversalTickPrice::Realtime(current_tick_price) => {
                (current_tick_price, current_tick_price)
            }
        };

        for level in working_level_store
            .get_created_working_levels()?
            .into_iter()
            .chain(working_level_store.get_active_working_levels()?.into_iter())
            .map(|level| Item {
                id: level.id,
                props: level.props.into(),
            })
        {
            let current_crossing_value = match level.props.r#type {
                OrderType::Buy => price_to_points(level.props.price - lowest_current_tick_price),
                OrderType::Sell => price_to_points(highest_current_tick_price - level.props.price),
            };

            log::debug!(
                "current crossing value of level ({:?}) is {}",
                level,
                current_crossing_value
            );

            if current_crossing_value > dec!(0) {
                match working_level_store.get_max_crossing_value_of_working_level(&level.id)? {
                    None => {
                        working_level_store.update_max_crossing_value_of_working_level(
                            &level.id,
                            current_crossing_value,
                        )?;

                        log::debug!(
                            "max crossing value of level ({:?}) is set to {}",
                            level,
                            current_crossing_value
                        );
                    }
                    Some(last_crossing_value) => {
                        log::debug!(
                            "last max crossing value of level ({:?}) is {}",
                            level,
                            last_crossing_value
                        );

                        if current_crossing_value > last_crossing_value {
                            working_level_store.update_max_crossing_value_of_working_level(
                                &level.id,
                                current_crossing_value,
                            )?;

                            log::debug!(
                                "max crossing value of level ({:?}) is updated to {}",
                                level,
                                current_crossing_value
                            );
                        } else {
                            log::debug!("max crossing value of level ({:?}) is not updated", level);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn remove_invalid_working_levels<W, A, D, M, C, E, T, N, O>(
        current_tick: &BasicTickProperties<UniversalTickPrice>,
        current_volatility: CandleVolatility,
        utils: RemoveInvalidWorkingLevelsUtils<W, A, D, M, C, E, T, O>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        mut entity: StatisticsNotifier<N>,
    ) -> Result<()>
    where
        T: AsRef<BasicWLProperties>,
        O: AsRef<BasicOrderProperties>,
        W: StepWorkingLevelStore<WorkingLevelProperties = T, OrderProperties = O>,
        A: Fn(&[O]) -> bool,
        D: Fn(WLPrice, UniversalTickPrice, ParamOutputValue) -> bool,
        M: Fn(LevelTime, TickTime, ParamOutputValue, &E) -> bool,
        C: Fn(&T, Option<WLMaxCrossingValue>, ParamOutputValue, UniversalTickPrice) -> bool,
        E: Fn(NaiveDateTime, NaiveDateTime, &[Holiday]) -> NumberOfDaysToExclude,
        N: NotificationQueue,
    {
        for level in utils.working_level_store.get_all_working_levels()? {
            let converted_level = Item {
                id: &level.id,
                props: level.props.as_ref(),
            };

            let level_status = utils
                .working_level_store
                .get_working_level_status(&level.id)?
                .unwrap();

            let mut remove_level = false;

            let distance_from_level_for_its_deletion = params.get_ratio_param_value(
                StepRatioParam::DistanceFromLevelForItsDeletion,
                current_volatility,
            );

            if level_status == WLStatus::Created
                || (level_status == WLStatus::Active
                    && (utils.level_has_no_active_orders)(
                        &utils
                            .working_level_store
                            .get_working_level_chain_of_orders(&level.id)?
                            .into_iter()
                            .map(|order| order.props)
                            .collect::<Vec<_>>(),
                    ))
            {
                if (utils.level_expired_by_distance)(
                    converted_level.props.price,
                    current_tick.bid,
                    distance_from_level_for_its_deletion,
                ) {
                    log::debug!("level ({:?}) is expired by distance", converted_level);

                    match &mut entity {
                        StatisticsNotifier::Backtesting(statistics) => {
                            statistics.deleted_by_expiration_by_distance += 1;
                        }
                        StatisticsNotifier::Realtime(queue) => {
                            queue.send_message(format!(
                                "level ({:?}) is expired by distance",
                                converted_level
                            ))?;
                        }
                    }

                    remove_level = true;
                } else {
                    log::debug!("level ({:?}) is NOT expired by distance", converted_level);

                    let level_expiration =
                        params.get_point_param_value(StepPointParam::LevelExpirationDays);

                    if (utils.level_expired_by_time)(
                        converted_level.props.time,
                        current_tick.time,
                        level_expiration,
                        utils.exclude_weekend_and_holidays,
                    ) {
                        log::debug!("level ({:?}) is expired by time", converted_level);

                        match &mut entity {
                            StatisticsNotifier::Backtesting(statistics) => {
                                statistics.deleted_by_expiration_by_time += 1;
                            }
                            StatisticsNotifier::Realtime(queue) => {
                                queue.send_message(format!(
                                    "level ({:?}) is expired by time",
                                    converted_level
                                ))?;
                            }
                        }

                        remove_level = true;
                    } else {
                        log::debug!("level ({:?}) is NOT expired by time", converted_level);

                        if level_status == WLStatus::Active {
                            let max_crossing_value = utils
                                .working_level_store
                                .get_max_crossing_value_of_working_level(&level.id)?;

                            let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion = params.get_ratio_param_value(
                                StepRatioParam::MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion,
                                current_volatility
                            );

                            if (utils.active_level_exceeds_activation_crossing_distance_when_returned_to_level)(
                                &level.props,
                                max_crossing_value,
                                min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
                                current_tick.bid
                            ) {
                                log::debug!(
                                    "level ({:?}) exceeds activation crossing distance when returned to level: {:?} >= {}",
                                    converted_level, max_crossing_value,
                                    min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion
                                );

                                match &mut entity {
                                    StatisticsNotifier::Backtesting(statistics) => {
                                        statistics.deleted_by_exceeding_activation_crossing_distance += 1;
                                    }
                                    StatisticsNotifier::Realtime(queue) => {
                                        queue.send_message(format!(
                                            "level ({:?}) exceeds activation crossing distance when returned to level: {:?} >= {}",
                                            converted_level, max_crossing_value,
                                            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion
                                        ))?;
                                    }
                                }

                                remove_level = true;
                            } else {
                                log::debug!(
                                    "level ({:?}) DOES NOT exceed activation crossing distance when returned to level: {:?} < {}",
                                    converted_level, max_crossing_value,
                                    min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion
                                );
                            }
                        }
                    }
                }
            }

            if remove_level {
                let chain = utils
                    .working_level_store
                    .get_working_level_chain_of_orders(&level.id)?;
                for order in chain {
                    if order.props.as_ref().status == OrderStatus::Opened {
                        dbg!(&order.props.as_ref());
                        dbg!(&level.props.as_ref());
                    }
                }
                utils.working_level_store.remove_working_level(&level.id)?;

                if let StatisticsNotifier::Backtesting(statistics) = &mut entity {
                    statistics.number_of_working_levels -= 1;
                }
            }
        }

        Ok(())
    }

    fn move_take_profits<W>(
        working_level_store: &mut impl StepWorkingLevelStore<WorkingLevelProperties = W>,
        distance_from_level_for_signaling_of_moving_take_profits: ParamOutputValue,
        distance_to_move_take_profits: ParamOutputValue,
        current_tick_price: UniversalTickPrice,
    ) -> Result<()>
    where
        W: Into<BasicWLProperties>,
    {
        let (lowest_current_tick_price, highest_current_tick_price) = match current_tick_price {
            UniversalTickPrice::Historical(current_tick_price) => {
                (current_tick_price.low, current_tick_price.high)
            }
            UniversalTickPrice::Realtime(current_tick_price) => {
                (current_tick_price, current_tick_price)
            }
        };

        for level in working_level_store
            .get_active_working_levels()?
            .into_iter()
            .map(|level| Item {
                id: level.id,
                props: level.props.into(),
            })
        {
            if !working_level_store.take_profits_of_level_are_moved(&level.id)? {
                let deviation_distance = match level.props.r#type {
                    OrderType::Buy => {
                        price_to_points(level.props.price - lowest_current_tick_price)
                    }
                    OrderType::Sell => {
                        price_to_points(highest_current_tick_price - level.props.price)
                    }
                };

                if deviation_distance >= distance_from_level_for_signaling_of_moving_take_profits {
                    log::debug!(
                        "move take profits of level ({:?}), because the deviation distance ({}) \
                    >= distance from level for signaling of moving take profits ({})",
                        level,
                        deviation_distance,
                        distance_from_level_for_signaling_of_moving_take_profits
                    );

                    working_level_store
                        .move_take_profits_of_level(&level.id, distance_to_move_take_profits)?;
                }
            }
        }

        Ok(())
    }

    fn update_tendency_and_get_instruction_to_create_new_working_level<
        S,
        D,
        A,
        C,
        N,
        H,
        B,
        P,
        M,
        K,
        X,
        L,
    >(
        config: &mut StepConfig,
        store: &mut S,
        utils: UpdateTendencyAndCreateWorkingLevelUtils<D, A, C, S, B, P, M, K, X, L>,
        mut statistics_charts_notifier: StatisticsChartsNotifier<N, H>,
        crossed_angle: &Item<AngleId, FullAngleProperties<A, C>>,
        current_candle: &Item<CandleId, C>,
        params: &M,
    ) -> Result<bool>
    where
        S: StepAngleStore<AngleProperties = A, CandleProperties = C>
            + StepCandleStore<CandleProperties = C>
            + StepWorkingLevelStore<WorkingLevelProperties = K>,
        D: Fn(&str, Option<&str>, bool, bool) -> bool,
        A: AsRef<BasicAngleProperties> + Debug,
        C: AsRef<StepCandleProperties> + Debug + PartialEq,
        N: NotificationQueue,
        H: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
        B: Fn(
            &Item<AngleId, FullAngleProperties<A, C>>,
            &[Item<CandleId, C>],
            &S,
            ParamOutputValue,
        ) -> Result<bool>,
        M: StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        P: Fn(
            &Item<AngleId, FullAngleProperties<A, C>>,
            &Item<CandleId, C>,
            &S,
            &M,
        ) -> Result<bool>,
        K: AsRef<BasicWLProperties>,
        X: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S) -> Result<bool>,
        L: Fn(&Item<AngleId, FullAngleProperties<A, C>>, &S, ParamOutputValue) -> Result<bool>,
    {
        let tendency_change_angle = store.get_tendency_change_angle()?;

        if config.tendency == Tendency::Unknown {
            log::debug!("previous tendency is unknown");

            config.tendency = crossed_angle.props.base.as_ref().r#type.into();

            log::debug!("tendency changed to {:?}", config.tendency);

            if let StatisticsChartsNotifier::Backtesting {
                add_entity_to_chart_traces,
                chart_traces,
                current_candle_chart_index,
                ..
            } = &mut statistics_charts_notifier
            {
                if Mode::from_str(&dotenv::var(MODE_ENV).unwrap()).unwrap() != Mode::Optimization {
                    add_entity_to_chart_traces(
                        ChartTraceEntity::Tendency(config.tendency),
                        chart_traces,
                        current_candle_chart_index.to_owned(),
                    );
                }
            }
        } else {
            let is_second_level_after_bargaining_tendency_change = (utils
                .is_second_level_after_bargaining_tendency_change)(
                &crossed_angle.id,
                tendency_change_angle
                    .as_ref()
                    .map(|angle| angle.id.as_str()),
                config.tendency_changed_on_crossing_bargaining_corridor,
                config.second_level_after_bargaining_tendency_change_is_created,
            );

            if config.tendency != crossed_angle.props.base.as_ref().r#type.into()
                || is_second_level_after_bargaining_tendency_change
                || (tendency_change_angle.is_some()
                    && tendency_change_angle.unwrap().id == crossed_angle.id)
            {
                let mut skip_creating_new_working_level = false;

                if config.tendency != crossed_angle.props.base.as_ref().r#type.into() {
                    config.tendency = crossed_angle.props.base.as_ref().r#type.into();

                    if let StatisticsChartsNotifier::Backtesting { statistics, .. } =
                        &mut statistics_charts_notifier
                    {
                        statistics.number_of_tendency_changes += 1;
                    }

                    log::debug!("tendency changed to {:?}", config.tendency);

                    store.update_tendency_change_angle(crossed_angle.id.clone())?;

                    log::debug!("set tendency change angle to {:?}", crossed_angle);

                    if let StatisticsChartsNotifier::Backtesting {
                        add_entity_to_chart_traces,
                        chart_traces,
                        current_candle_chart_index,
                        ..
                    } = &mut statistics_charts_notifier
                    {
                        if Mode::from_str(&dotenv::var(MODE_ENV).unwrap()).unwrap()
                            != Mode::Optimization
                        {
                            add_entity_to_chart_traces(
                                ChartTraceEntity::Tendency(config.tendency),
                                chart_traces,
                                current_candle_chart_index.to_owned(),
                            );
                        }
                    }

                    store.update_angle_of_second_level_after_bargaining_tendency_change(None)?;

                    log::debug!(
                        "set angle of second level after bargaining tendency change to None"
                    );

                    config.second_level_after_bargaining_tendency_change_is_created = false;

                    log::debug!(
                        "set second_level_after_bargaining_tendency_change_is_created to false"
                    );

                    if !(utils.level_comes_out_of_bargaining_corridor)(
                        crossed_angle,
                        &store.get_candles_of_general_corridor()?,
                        store,
                        params.get_point_param_value(
                            StepPointParam::MinAmountOfCandlesInCorridorDefiningEdgeBargaining,
                        ),
                    )? {
                        skip_creating_new_working_level = false;

                        log::debug!("set skip_creating_new_working_level to false");

                        config.tendency_changed_on_crossing_bargaining_corridor = false;

                        log::debug!(
                            "set tendency_changed_on_crossing_bargaining_corridor to false"
                        );
                    } else {
                        skip_creating_new_working_level = true;

                        log::debug!("set skip_creating_new_working_level to true");

                        config.tendency_changed_on_crossing_bargaining_corridor = true;

                        log::debug!("set tendency_changed_on_crossing_bargaining_corridor to true");

                        match crossed_angle.props.base.as_ref().r#type {
                            Level::Min => {
                                if let Some(min_angle_before_bargaining_corridor) =
                                    store.get_min_angle_before_bargaining_corridor()?
                                {
                                    store.update_min_angle(
                                        min_angle_before_bargaining_corridor.id.clone(),
                                    )?;

                                    log::debug!("set back the min angle to be the min angle before bargaining corridor: new min angle: {min_angle_before_bargaining_corridor:?}");
                                } else {
                                    log::debug!("min angle before bargaining corridor is None, so it cannot be set as the min angle");
                                }
                            }
                            Level::Max => {
                                if let Some(max_angle_before_bargaining_corridor) =
                                    store.get_max_angle_before_bargaining_corridor()?
                                {
                                    store.update_max_angle(
                                        max_angle_before_bargaining_corridor.id.clone(),
                                    )?;

                                    log::debug!("set back the max angle to be the max angle before bargaining corridor: new max angle: {max_angle_before_bargaining_corridor:?}");
                                } else {
                                    log::debug!("max angle before bargaining corridor is None, so it cannot be set as the max angle");
                                }
                            }
                        }
                    }
                } else if is_second_level_after_bargaining_tendency_change {
                    match store.get_angle_of_second_level_after_bargaining_tendency_change()? {
                        None => {
                            store.update_angle_of_second_level_after_bargaining_tendency_change(
                                Some(crossed_angle.id.clone()),
                            )?;

                            log::debug!("set angle_of_second_level_after_bargaining_tendency_change to {crossed_angle:?}");

                            skip_creating_new_working_level = false;

                            log::debug!("set skip_creating_new_working_level to false");
                        }
                        Some(angle_of_second_level_after_bargaining_tendency_change) => {
                            if crossed_angle.id
                                != angle_of_second_level_after_bargaining_tendency_change.id
                            {
                                skip_creating_new_working_level = true;

                                log::debug!(
                                    "set skip_creating_new_working_level to true, because the crossed angle \
                                    is NOT the angle of second level after bargaining tendency change: \
                                    crossed angle: {crossed_angle:?}, angle of second level after bargaining \
                                    tendency change: {angle_of_second_level_after_bargaining_tendency_change:?}" 
                                );
                            }
                        }
                    }
                }

                if !skip_creating_new_working_level
                    && !(utils.working_level_exists)(crossed_angle, store)?
                    && (utils.appropriate_working_level)(
                        crossed_angle,
                        current_candle,
                        store,
                        params,
                    )?
                    && !(utils.working_level_is_close_to_another_one)(
                        crossed_angle,
                        store,
                        params.get_ratio_param_value(
                            StepRatioParam::DistanceDefiningNearbyLevelsOfTheSameType,
                            current_candle.props.as_ref().base.volatility,
                        ),
                    )?
                {
                    if is_second_level_after_bargaining_tendency_change {
                        config.second_level_after_bargaining_tendency_change_is_created = true;

                        log::debug!(
                            "set second_level_after_bargaining_tendency_change_is_created to true"
                        );
                    }

                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests;
