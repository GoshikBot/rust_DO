use crate::step::utils::backtesting_charts::{ChartTraceEntity, ChartTracesModifier};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::entities::working_levels::{BacktestingWLProperties, CorridorType};
use crate::step::utils::entities::{Mode, MODE_ENV};
use crate::step::utils::level_conditions::LevelConditions;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use crate::step::utils::stores::{StepBacktestingConfig, StepBacktestingStatistics};
use anyhow::{bail, Result};
use backtesting::trading_engine::TradingEngine;
use backtesting::{Balance, ClosePositionBy, OpenPositionBy};
use base::entities::order::{
    BasicOrderPrices, BasicOrderProperties, OrderStatus, OrderType, OrderVolume,
};
use base::entities::{
    BasicTickProperties, PRICE_DECIMAL_PLACES, TARGET_LOGGER_ENV, VOLUME_DECIMAL_PLACES,
};
use base::stores::order_store::BasicOrderStore;
use base::{
    entities::{candle::CandleVolatility, Item, LOT},
    helpers::points_to_price,
    params::StrategyParams,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::str::FromStr;

use super::entities::{
    order::StepOrderProperties,
    params::{StepPointParam, StepRatioParam},
    working_levels::{BasicWLProperties, WLId},
};

pub trait OrderUtils {
    /// Creates the chain of orders from the particular level when this level is crossed.
    fn get_new_chain_of_orders<W>(
        &self,
        level: &Item<WLId, W>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        current_volatility: CandleVolatility,
        current_balance: Balance,
    ) -> Result<Vec<StepOrderProperties>>
    where
        W: AsRef<BasicWLProperties>;

    /// Places and closed orders.
    fn update_orders_backtesting<M, T, C, L>(
        &self,
        current_tick: &BasicTickProperties,
        current_candle: &StepBacktestingCandleProperties,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        stores: UpdateOrdersBacktestingStores<M>,
        utils: UpdateOrdersBacktestingUtils<T, C, L>,
        no_trading_mode: bool,
    ) -> Result<()>
    where
        M: BasicOrderStore<OrderProperties = StepOrderProperties>
            + StepWorkingLevelStore<WorkingLevelProperties = BacktestingWLProperties>,
        T: TradingEngine,
        C: ChartTracesModifier,
        L: LevelConditions;
}

#[derive(Default)]
pub struct OrderUtilsImpl;

impl OrderUtilsImpl {
    pub fn new() -> Self {
        Self::default()
    }

    /// Converts the max loss per the chain of orders from percent of the balance to the real price.
    fn get_max_loss_per_chain_of_orders_in_price(
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        current_balance: Balance,
    ) -> Result<MaxLossPerChainOfOrders> {
        if current_balance <= dec!(0) {
            bail!("balance should be positive, but got {}", current_balance);
        }

        let max_loss_per_chain_of_orders_pct = current_balance
            * params.get_point_param_value(StepPointParam::MaxLossPerOneChainOfOrdersPctOfBalance)
            / dec!(100);

        log::debug!(
            target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
            "max loss per chain of orders in price is {}; current balance — {}",
            max_loss_per_chain_of_orders_pct, current_balance
        );

        Ok(max_loss_per_chain_of_orders_pct)
    }

    /// Calculates the volume per order based on the max loss per the chain of orders.
    fn get_volume_per_order(
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        distance_between_orders: DistanceBetweenOrders,
        current_balance: Balance,
    ) -> Result<OrderVolume> {
        let max_loss = Self::get_max_loss_per_chain_of_orders_in_price(params, current_balance)?;

        let amount_of_orders = params.get_point_param_value(StepPointParam::AmountOfOrders);

        let volume_per_order: Decimal = max_loss * dec!(2)
            / (amount_of_orders
                * (dec!(2) + amount_of_orders - dec!(1))
                * distance_between_orders
                * Decimal::from(LOT));

        log::debug!(
            target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
            "volume per order — {}",
            volume_per_order
        );

        Ok(volume_per_order.round_dp(VOLUME_DECIMAL_PLACES))
    }
}

impl OrderUtils for OrderUtilsImpl {
    fn get_new_chain_of_orders<W>(
        &self,
        level: &Item<WLId, W>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        current_volatility: CandleVolatility,
        current_balance: Balance,
    ) -> Result<Vec<StepOrderProperties>>
    where
        W: AsRef<BasicWLProperties>,
    {
        let distance_from_level_to_first_order = points_to_price(params.get_ratio_param_value(
            StepRatioParam::DistanceFromLevelToFirstOrder,
            current_volatility,
        ));

        let distance_from_level_to_stop_loss = points_to_price(params.get_ratio_param_value(
            StepRatioParam::DistanceFromLevelToStopLoss,
            current_volatility,
        ));

        let distance_between_orders = (distance_from_level_to_stop_loss
            - distance_from_level_to_first_order)
            / params.get_point_param_value(StepPointParam::AmountOfOrders);

        let volume_per_order =
            Self::get_volume_per_order(params, distance_between_orders, current_balance)?;

        let (mut price_for_current_order, stop_loss) = match level.props.as_ref().r#type {
            OrderType::Buy => {
                let price_for_current_order =
                    level.props.as_ref().price - distance_from_level_to_first_order;
                let stop_loss = level.props.as_ref().price - distance_from_level_to_stop_loss;
                (
                    price_for_current_order.round_dp(PRICE_DECIMAL_PLACES),
                    stop_loss.round_dp(PRICE_DECIMAL_PLACES),
                )
            }
            OrderType::Sell => {
                let price_for_current_order =
                    level.props.as_ref().price + distance_from_level_to_first_order;
                let stop_loss = level.props.as_ref().price + distance_from_level_to_stop_loss;
                (
                    price_for_current_order.round_dp(PRICE_DECIMAL_PLACES),
                    stop_loss.round_dp(PRICE_DECIMAL_PLACES),
                )
            }
        };

        let take_profit = level.props.as_ref().price.round_dp(PRICE_DECIMAL_PLACES);

        let mut chain_of_orders = Vec::new();

        let amount_of_orders = params
            .get_point_param_value(StepPointParam::AmountOfOrders)
            .normalize()
            .to_string()
            .parse::<usize>()
            .unwrap();

        for _ in 0..amount_of_orders {
            chain_of_orders.push(StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: level.props.as_ref().r#type,
                    volume: volume_per_order,
                    status: Default::default(),
                    prices: BasicOrderPrices {
                        open: price_for_current_order,
                        stop_loss,
                        take_profit,
                    },
                },
                working_level_id: level.id.clone(),
            });

            match level.props.as_ref().r#type {
                OrderType::Buy => price_for_current_order -= distance_between_orders,
                OrderType::Sell => price_for_current_order += distance_between_orders,
            }

            price_for_current_order = price_for_current_order.round_dp(PRICE_DECIMAL_PLACES);
        }

        Ok(chain_of_orders)
    }

    fn update_orders_backtesting<M, T, C, L>(
        &self,
        current_tick: &BasicTickProperties,
        current_candle: &StepBacktestingCandleProperties,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        stores: UpdateOrdersBacktestingStores<M>,
        utils: UpdateOrdersBacktestingUtils<T, C, L>,
        no_trading_mode: bool,
    ) -> Result<()>
    where
        M: BasicOrderStore<OrderProperties = StepOrderProperties>
            + StepWorkingLevelStore<WorkingLevelProperties = BacktestingWLProperties>,

        T: TradingEngine,
        C: ChartTracesModifier,
        L: LevelConditions,
    {
        for order in stores.main.get_all_orders()? {
            match order.props.base.status {
                OrderStatus::Pending => {
                    if (order.props.base.r#type == OrderType::Buy
                        && current_tick.bid <= order.props.base.prices.open)
                        || (order.props.base.r#type == OrderType::Sell
                            && current_tick.bid >= order.props.base.prices.open)
                    {
                        let mut remove_working_level = false;

                        if !utils.level_conditions.level_exceeds_amount_of_candles_in_corridor(
                            &order.props.working_level_id,
                            stores.main,
                            CorridorType::Small,
                            params.get_point_param_value(StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel),
                        )? {
                            if !utils.level_conditions.level_exceeds_amount_of_candles_in_corridor(
                                &order.props.working_level_id,
                                stores.main,
                                CorridorType::Big,
                                params.get_point_param_value(StepPointParam::MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel),
                            )? {
                                if !utils.level_conditions.price_is_beyond_stop_loss(
                                    current_tick.bid,
                                    order.props.base.prices.stop_loss,
                                    order.props.base.r#type,
                                ) {
                                    if !no_trading_mode {
                                        utils.trading_engine.open_position(&order, OpenPositionBy::OpenPrice, stores.main, &mut stores.config.trading_engine)?;
                                    }
                                } else {
                                    stores.statistics.deleted_by_price_being_beyond_stop_loss += 1;
                                    remove_working_level = true;
                                }
                            } else {
                                stores.statistics.deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing += 1;
                                remove_working_level = true;
                            }
                        } else {
                            stores.statistics.deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing += 1;
                            remove_working_level = true;
                        }

                        if remove_working_level {
                            stores
                                .main
                                .remove_working_level(&order.props.working_level_id)?;
                            stores.statistics.number_of_working_levels -= 1;
                        }
                    }
                }
                OrderStatus::Opened => {
                    let mut add_to_chart_traces = false;

                    if (order.props.base.r#type == OrderType::Buy
                        && current_tick.bid >= order.props.base.prices.take_profit)
                        || (order.props.base.r#type == OrderType::Sell
                            && current_tick.bid <= order.props.base.prices.take_profit)
                    {
                        add_to_chart_traces = true;
                        utils.trading_engine.close_position(
                            &order,
                            ClosePositionBy::TakeProfit,
                            stores.main,
                            &mut stores.config.trading_engine,
                        )?;
                    } else if (order.props.base.r#type == OrderType::Buy
                        && current_tick.bid <= order.props.base.prices.stop_loss)
                        || (order.props.base.r#type == OrderType::Sell
                            && current_tick.bid >= order.props.base.prices.stop_loss)
                    {
                        add_to_chart_traces = true;
                        utils.trading_engine.close_position(
                            &order,
                            ClosePositionBy::StopLoss,
                            stores.main,
                            &mut stores.config.trading_engine,
                        )?;
                    }

                    let working_level_chart_index = stores
                        .main
                        .get_working_level_by_id(&order.props.working_level_id)?
                        .unwrap()
                        .props
                        .chart_index;

                    if add_to_chart_traces
                        && Mode::from_str(&dotenv::var(MODE_ENV).unwrap()).unwrap() == Mode::Debug
                    {
                        utils.chart_traces_modifier.add_entity_to_chart_traces(
                            ChartTraceEntity::TakeProfit {
                                take_profit_price: order.props.base.prices.take_profit,
                                working_level_chart_index,
                            },
                            &mut stores.config.traces,
                            current_candle,
                        );

                        utils.chart_traces_modifier.add_entity_to_chart_traces(
                            ChartTraceEntity::StopLoss {
                                stop_loss_price: order.props.base.prices.stop_loss,
                                working_level_chart_index,
                            },
                            &mut stores.config.traces,
                            current_candle,
                        );
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }
}

type MaxLossPerChainOfOrders = Decimal;

type DistanceBetweenOrders = Decimal;

pub struct UpdateOrdersBacktestingUtils<'a, T, C, L>
where
    T: TradingEngine,
    C: ChartTracesModifier,
    L: LevelConditions,
{
    pub trading_engine: &'a T,
    pub chart_traces_modifier: &'a C,
    pub level_conditions: &'a L,
}

pub struct UpdateOrdersBacktestingStores<'a, M>
where
    M: BasicOrderStore<OrderProperties = StepOrderProperties>
        + StepWorkingLevelStore<WorkingLevelProperties = BacktestingWLProperties>,
{
    pub main: &'a mut M,
    pub config: &'a mut StepBacktestingConfig,
    pub statistics: &'a mut StepBacktestingStatistics,
}

#[cfg(test)]
mod tests {
    use crate::step::utils::backtesting_charts::StepBacktestingChartTraces;
    use crate::step::utils::entities::working_levels::{
        LevelTime, WLMaxCrossingValue, WLPrice, WLStatus,
    };
    use crate::step::utils::level_conditions::MinAmountOfCandles;
    use backtesting::BacktestingTradingEngineConfig;
    use base::entities::candle::CandleId;
    use base::entities::order::{OrderId, OrderPrice};
    use base::entities::tick::{TickPrice, TickTime};
    use base::helpers::{Holiday, NumberOfDaysToExclude};
    use base::params::ParamValue;
    use chrono::{NaiveDateTime, Utc};
    use rust_decimal_macros::dec;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::env;

    use super::*;

    #[derive(Default)]
    struct StepTestParams {}

    impl StepTestParams {
        fn new() -> Self {
            Default::default()
        }
    }

    impl StrategyParams for StepTestParams {
        type PointParam = StepPointParam;
        type RatioParam = StepRatioParam;

        fn get_point_param_value(&self, name: Self::PointParam) -> ParamValue {
            match name {
                StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct => unreachable!(),
                StepPointParam::AmountOfOrders => dec!(5.0),
                StepPointParam::LevelExpirationDays => unreachable!(),
                StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel => unreachable!(),
                StepPointParam::MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel => unreachable!(),
                StepPointParam::MinAmountOfCandlesInCorridorDefiningEdgeBargaining => unreachable!(),
                StepPointParam::MaxLossPerOneChainOfOrdersPctOfBalance => dec!(10.0)
            }
        }

        fn get_ratio_param_value(
            &self,
            name: Self::RatioParam,
            volatility: CandleVolatility,
        ) -> ParamValue {
            let value = match name {
                StepRatioParam::MinDistanceBetweenMaxMinAngles => unreachable!(),
                StepRatioParam::MaxDistanceBetweenMaxMinAnglesForTheirUpdating => unreachable!(),
                StepRatioParam::MinBreakDistance => unreachable!(),
                StepRatioParam::DistanceFromLevelToFirstOrder => dec!(0.7),
                StepRatioParam::DistanceFromLevelToStopLoss => dec!(3.6),
                StepRatioParam::DistanceFromLevelForSignalingOfMovingTakeProfits => unreachable!(),
                StepRatioParam::DistanceToMoveTakeProfits => unreachable!(),
                StepRatioParam::DistanceFromLevelForItsDeletion => unreachable!(),
                StepRatioParam::DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel => unreachable!(),
                StepRatioParam::DistanceDefiningNearbyLevelsOfTheSameType => unreachable!(),
                StepRatioParam::MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion => unreachable!(),
                StepRatioParam::BigCorridorNearLevel => unreachable!(),
            };

            value * Decimal::from(volatility)
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_new_chain_of_orders__positive_balance__should_return_correct_chain_of_orders() {
        let level = Item {
            id: String::from("1"),
            props: BasicWLProperties {
                price: dec!(1.3),
                r#type: OrderType::Buy,
                time: Utc::now().naive_utc(),
            },
        };

        let params = StepTestParams::new();

        let volatility = 180;
        let balance = dec!(400);

        let expected_chain_of_orders = vec![
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    volume: dec!(0.03),
                    status: Default::default(),
                    prices: BasicOrderPrices {
                        open: dec!(1.29874),
                        stop_loss: dec!(1.29352),
                        take_profit: dec!(1.3),
                    },
                },
                working_level_id: String::from("1"),
            },
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    volume: dec!(0.03),
                    status: Default::default(),
                    prices: BasicOrderPrices {
                        open: dec!(1.29770),
                        stop_loss: dec!(1.29352),
                        take_profit: dec!(1.3),
                    },
                },
                working_level_id: String::from("1"),
            },
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    volume: dec!(0.03),
                    status: Default::default(),
                    prices: BasicOrderPrices {
                        open: dec!(1.29666),
                        stop_loss: dec!(1.29352),
                        take_profit: dec!(1.3),
                    },
                },
                working_level_id: String::from("1"),
            },
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    volume: dec!(0.03),
                    status: Default::default(),
                    prices: BasicOrderPrices {
                        open: dec!(1.29562),
                        stop_loss: dec!(1.29352),
                        take_profit: dec!(1.3),
                    },
                },
                working_level_id: String::from("1"),
            },
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    volume: dec!(0.03),
                    status: Default::default(),
                    prices: BasicOrderPrices {
                        open: dec!(1.29458),
                        stop_loss: dec!(1.29352),
                        take_profit: dec!(1.3),
                    },
                },
                working_level_id: String::from("1"),
            },
        ];

        let order_utils = OrderUtilsImpl::new();

        let chain_of_orders = order_utils
            .get_new_chain_of_orders(&level, &params, volatility, balance)
            .unwrap();

        assert_eq!(chain_of_orders, expected_chain_of_orders);
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_new_chain_of_orders__zero_balance__should_return_error_result() {
        let level = Item {
            id: String::from("1"),
            props: BasicWLProperties {
                price: dec!(1.3),
                r#type: OrderType::Buy,
                time: Utc::now().naive_utc(),
            },
        };

        let params = StepTestParams::new();

        let volatility = 180;
        let balance = dec!(0);

        let order_utils = OrderUtilsImpl::new();

        let chain_of_orders =
            order_utils.get_new_chain_of_orders(&level, &params, volatility, balance);

        assert!(chain_of_orders.is_err());
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_new_chain_of_orders__negative_balance__should_return_error_result() {
        let level = Item {
            id: String::from("1"),
            props: BasicWLProperties {
                price: dec!(1.3),
                r#type: OrderType::Buy,
                time: Utc::now().naive_utc(),
            },
        };

        let params = StepTestParams::new();

        let volatility = 180;
        let balance = dec!(-10);

        let order_utils = OrderUtilsImpl::new();

        let chain_of_orders =
            order_utils.get_new_chain_of_orders(&level, &params, volatility, balance);

        assert!(chain_of_orders.is_err());
    }

    #[derive(Default)]
    struct TestLevelConditions {
        level_exceeds_amount_of_candles_small_corridor_number_of_calls: RefCell<u32>,
        level_exceeds_amount_of_candles_big_corridor_number_of_calls: RefCell<u32>,
        price_is_beyond_stop_loss_number_of_calls: RefCell<u32>,
    }

    impl LevelConditions for TestLevelConditions {
        fn level_exceeds_amount_of_candles_in_corridor(
            &self,
            level_id: &str,
            _working_level_store: &impl StepWorkingLevelStore,
            corridor_type: CorridorType,
            _min_amount_of_candles: MinAmountOfCandles,
        ) -> Result<bool> {
            match corridor_type {
                CorridorType::Small => {
                    *self
                        .level_exceeds_amount_of_candles_small_corridor_number_of_calls
                        .borrow_mut() += 1
                }
                CorridorType::Big => {
                    *self
                        .level_exceeds_amount_of_candles_big_corridor_number_of_calls
                        .borrow_mut() += 1
                }
            }

            match level_id {
                "2" | "4" | "5" | "7" | "9" | "10" if corridor_type == CorridorType::Small => {
                    Ok(false)
                }
                "2" | "5" | "7" | "10" if corridor_type == CorridorType::Big => Ok(false),
                _ => Ok(true),
            }
        }

        fn price_is_beyond_stop_loss(
            &self,
            _current_tick_price: TickPrice,
            stop_loss_price: OrderPrice,
            _working_level_type: OrderType,
        ) -> bool {
            *self.price_is_beyond_stop_loss_number_of_calls.borrow_mut() += 1;
            stop_loss_price != dec!(1.88888)
        }

        fn level_expired_by_distance(
            &self,
            _level_price: WLPrice,
            _current_tick_price: TickPrice,
            _distance_from_level_for_its_deletion: ParamValue,
        ) -> bool {
            unimplemented!()
        }

        fn level_expired_by_time(
            &self,
            level_time: LevelTime,
            current_tick_time: TickTime,
            level_expiration: ParamValue,
            exclude_weekend_and_holidays: &impl Fn(
                NaiveDateTime,
                NaiveDateTime,
                &[Holiday],
            ) -> NumberOfDaysToExclude,
        ) -> bool {
            unimplemented!()
        }

        fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &self,
            level: &impl AsRef<BasicWLProperties>,
            max_crossing_value: Option<WLMaxCrossingValue>,
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
            current_tick_price: TickPrice,
        ) -> bool {
            unimplemented!()
        }

        fn level_has_no_active_orders<T>(&self, level_orders: &[T]) -> bool
        where
            T: AsRef<BasicOrderProperties>,
        {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct TestParams;

    impl StrategyParams for TestParams {
        type PointParam = StepPointParam;
        type RatioParam = StepRatioParam;

        fn get_point_param_value(&self, _name: Self::PointParam) -> ParamValue {
            dec!(1)
        }

        fn get_ratio_param_value(
            &self,
            _name: Self::RatioParam,
            _volatility: CandleVolatility,
        ) -> ParamValue {
            dec!(1)
        }
    }

    #[derive(Default)]
    struct TestTradingEngine {
        opened_orders: RefCell<Vec<String>>,
        closed_orders_by_take_profit: RefCell<Vec<String>>,
        closed_orders_by_stop_loss: RefCell<Vec<String>>,
    }

    impl TradingEngine for TestTradingEngine {
        fn open_position<O>(
            &self,
            order: &Item<OrderId, O>,
            _by: OpenPositionBy,
            _order_store: &mut impl BasicOrderStore,
            _trading_config: &mut BacktestingTradingEngineConfig,
        ) -> Result<()>
        where
            O: Into<BasicOrderProperties> + Clone,
        {
            self.opened_orders.borrow_mut().push(order.id.clone());
            Ok(())
        }

        fn close_position<O>(
            &self,
            order: &Item<OrderId, O>,
            by: ClosePositionBy,
            _order_store: &mut impl BasicOrderStore<OrderProperties = O>,
            _trading_config: &mut BacktestingTradingEngineConfig,
        ) -> Result<()>
        where
            O: Into<BasicOrderProperties> + Clone,
        {
            match by {
                ClosePositionBy::TakeProfit => self
                    .closed_orders_by_take_profit
                    .borrow_mut()
                    .push(order.id.clone()),
                ClosePositionBy::StopLoss => self
                    .closed_orders_by_stop_loss
                    .borrow_mut()
                    .push(order.id.clone()),
                _ => unreachable!(),
            }

            Ok(())
        }
    }

    #[derive(Default)]
    struct TestChartTracesModifier {
        number_of_stop_loss_entities: RefCell<u32>,
        number_of_take_profit_entities: RefCell<u32>,
    }

    impl ChartTracesModifier for TestChartTracesModifier {
        fn add_entity_to_chart_traces(
            &self,
            entity: ChartTraceEntity,
            _chart_traces: &mut StepBacktestingChartTraces,
            _current_candle: &StepBacktestingCandleProperties,
        ) {
            match entity {
                ChartTraceEntity::StopLoss { .. } => {
                    *self.number_of_stop_loss_entities.borrow_mut() += 1
                }
                ChartTraceEntity::TakeProfit { .. } => {
                    *self.number_of_take_profit_entities.borrow_mut() += 1
                }
                _ => unreachable!(),
            }
        }
    }

    struct TestStore {
        orders: Vec<Item<OrderId, <Self as BasicOrderStore>::OrderProperties>>,

        working_levels:
            HashMap<WLId, Item<WLId, <Self as StepWorkingLevelStore>::WorkingLevelProperties>>,

        removed_levels: HashSet<WLId>,
    }

    impl TestStore {
        fn new(
            orders: Vec<Item<OrderId, <Self as BasicOrderStore>::OrderProperties>>,
            working_levels_list: Vec<
                Item<WLId, <Self as StepWorkingLevelStore>::WorkingLevelProperties>,
            >,
        ) -> Self {
            let mut working_levels = HashMap::new();
            for working_level in working_levels_list {
                working_levels.insert(working_level.id.clone(), working_level);
            }

            Self {
                working_levels,
                removed_levels: Default::default(),
                orders,
            }
        }
    }

    impl BasicOrderStore for TestStore {
        type OrderProperties = StepOrderProperties;

        fn create_order(
            &mut self,
            _properties: Self::OrderProperties,
        ) -> Result<Item<OrderId, Self::OrderProperties>> {
            unimplemented!()
        }

        fn get_order_by_id(
            &self,
            _id: &str,
        ) -> Result<Option<Item<OrderId, Self::OrderProperties>>> {
            unimplemented!()
        }

        fn get_all_orders(&self) -> Result<Vec<Item<OrderId, Self::OrderProperties>>> {
            Ok(self.orders.clone())
        }

        fn update_order_status(&mut self, _order_id: &str, _new_status: OrderStatus) -> Result<()> {
            unimplemented!()
        }
    }

    impl StepWorkingLevelStore for TestStore {
        type WorkingLevelProperties = BacktestingWLProperties;
        type CandleProperties = ();
        type OrderProperties = ();

        fn create_working_level(
            &mut self,
            _properties: Self::WorkingLevelProperties,
        ) -> Result<Item<WLId, Self::WorkingLevelProperties>> {
            unimplemented!()
        }

        fn get_working_level_by_id(
            &self,
            id: &str,
        ) -> Result<Option<Item<WLId, Self::WorkingLevelProperties>>> {
            Ok(self.working_levels.get(id).cloned())
        }

        fn move_working_level_to_active(&mut self, _id: &str) -> Result<()> {
            unimplemented!()
        }

        fn remove_working_level(&mut self, id: &str) -> Result<()> {
            self.removed_levels.insert(id.to_string());
            Ok(())
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
            _corridor_type: CorridorType,
        ) -> Result<Vec<Item<CandleId, Self::CandleProperties>>> {
            unimplemented!()
        }

        fn update_max_crossing_value_of_working_level(
            &mut self,
            _working_level_id: &str,
            _new_value: WLMaxCrossingValue,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_max_crossing_value_of_working_level(
            &self,
            _working_level_id: &str,
        ) -> Result<Option<WLMaxCrossingValue>> {
            unimplemented!()
        }

        fn move_take_profits_of_level(&mut self, _working_level_id: &str) -> Result<()> {
            unimplemented!()
        }

        fn are_take_profits_of_level_moved(&self, _working_level_id: &str) -> Result<bool> {
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

    fn testing_working_levels() -> Vec<Item<WLId, BacktestingWLProperties>> {
        vec![
            Item {
                id: String::from("1"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 1,
                },
            },
            Item {
                id: String::from("2"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 2,
                },
            },
            Item {
                id: String::from("3"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 3,
                },
            },
            Item {
                id: String::from("4"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 4,
                },
            },
            Item {
                id: String::from("5"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 5,
                },
            },
            Item {
                id: String::from("6"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 6,
                },
            },
            Item {
                id: String::from("7"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 7,
                },
            },
            Item {
                id: String::from("8"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 8,
                },
            },
            Item {
                id: String::from("9"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 9,
                },
            },
            Item {
                id: String::from("10"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 10,
                },
            },
            Item {
                id: String::from("11"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 11,
                },
            },
            Item {
                id: String::from("12"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 12,
                },
            },
            Item {
                id: String::from("13"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 13,
                },
            },
            Item {
                id: String::from("14"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 14,
                },
            },
            Item {
                id: String::from("15"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 15,
                },
            },
            Item {
                id: String::from("16"),
                props: BacktestingWLProperties {
                    base: Default::default(),
                    chart_index: 16,
                },
            },
        ]
    }

    fn testing_orders() -> Vec<Item<OrderId, StepOrderProperties>> {
        vec![
            Item {
                id: String::from("1"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.28000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "1".to_string(),
                },
            },
            Item {
                id: String::from("2"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.30100),
                            stop_loss: dec!(1.88888),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "2".to_string(),
                },
            },
            Item {
                id: String::from("3"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.30100),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "3".to_string(),
                },
            },
            Item {
                id: String::from("4"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.30100),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "4".to_string(),
                },
            },
            Item {
                id: String::from("5"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.30100),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "5".to_string(),
                },
            },
            Item {
                id: String::from("6"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.30200),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "6".to_string(),
                },
            },
            Item {
                id: String::from("7"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.29000),
                            stop_loss: dec!(1.88888),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "7".to_string(),
                },
            },
            Item {
                id: String::from("8"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.29000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "8".to_string(),
                },
            },
            Item {
                id: String::from("9"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.29000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "9".to_string(),
                },
            },
            Item {
                id: String::from("10"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Pending,
                        prices: BasicOrderPrices {
                            open: dec!(1.29000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "10".to_string(),
                },
            },
            Item {
                id: String::from("11"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Opened,
                        prices: BasicOrderPrices {
                            take_profit: dec!(1.30100),
                            stop_loss: dec!(1.29000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "11".to_string(),
                },
            },
            Item {
                id: String::from("12"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Opened,
                        prices: BasicOrderPrices {
                            take_profit: dec!(1.29000),
                            stop_loss: dec!(1.28900),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "12".to_string(),
                },
            },
            Item {
                id: String::from("13"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Buy,
                        status: OrderStatus::Opened,
                        prices: BasicOrderPrices {
                            take_profit: dec!(1.32000),
                            stop_loss: dec!(1.31000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "13".to_string(),
                },
            },
            Item {
                id: String::from("14"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Opened,
                        prices: BasicOrderPrices {
                            take_profit: dec!(1.29000),
                            stop_loss: dec!(1.31000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "14".to_string(),
                },
            },
            Item {
                id: String::from("15"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Opened,
                        prices: BasicOrderPrices {
                            take_profit: dec!(1.32000),
                            stop_loss: dec!(1.31000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "15".to_string(),
                },
            },
            Item {
                id: String::from("16"),
                props: StepOrderProperties {
                    base: BasicOrderProperties {
                        r#type: OrderType::Sell,
                        status: OrderStatus::Opened,
                        prices: BasicOrderPrices {
                            take_profit: dec!(1.27000),
                            stop_loss: dec!(1.28000),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    working_level_id: "16".to_string(),
                },
            },
        ]
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_orders_backtesting__debug_and_allow_trading_mode__should_successfully_place_and_close_particular_orders(
    ) {
        // Notation
        // o — crossed open price
        // s — exceed amount of candles in small corridor
        // b — exceed amount of candles in big corridor
        // p — price is beyond stop loss

        // Working level indexes
        // 2(buy), 7(sell)  — o && !s && !b && !p
        // 3(buy), 8(sell)  — !o
        // 4(buy), 9(sell)  — o && s
        // 5(buy), 10(sell) — o && !s && b

        let current_tick = BasicTickProperties {
            bid: dec!(1.30000),
            ..Default::default()
        };

        let current_candle = StepBacktestingCandleProperties::default();

        let params = TestParams::default();

        let mut store = TestStore::new(testing_orders(), testing_working_levels());

        let mut config = StepBacktestingConfig::default(50);
        let mut statistics = StepBacktestingStatistics {
            number_of_working_levels: 16,
            ..Default::default()
        };

        let stores = UpdateOrdersBacktestingStores {
            main: &mut store,
            config: &mut config,
            statistics: &mut statistics,
        };

        let trading_engine = TestTradingEngine::default();
        let chart_traces_modifier = TestChartTracesModifier::default();
        let level_conditions = TestLevelConditions::default();

        let utils = UpdateOrdersBacktestingUtils {
            trading_engine: &trading_engine,
            chart_traces_modifier: &chart_traces_modifier,
            level_conditions: &level_conditions,
        };

        let no_trading_mode = false;

        env::set_var("MODE", "debug");

        let order_utils = OrderUtilsImpl::new();

        order_utils
            .update_orders_backtesting(
                &current_tick,
                &current_candle,
                &params,
                stores,
                utils,
                no_trading_mode,
            )
            .unwrap();

        assert_eq!(
            *level_conditions
                .level_exceeds_amount_of_candles_small_corridor_number_of_calls
                .borrow(),
            8
        );
        assert_eq!(statistics.deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing, 2);

        assert_eq!(
            *level_conditions
                .level_exceeds_amount_of_candles_big_corridor_number_of_calls
                .borrow(),
            6
        );
        assert_eq!(
            statistics
                .deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing,
            2
        );

        assert_eq!(
            *level_conditions
                .price_is_beyond_stop_loss_number_of_calls
                .borrow(),
            4
        );
        assert_eq!(statistics.deleted_by_price_being_beyond_stop_loss, 2);

        assert_eq!(
            *trading_engine.opened_orders.borrow(),
            vec![String::from("2"), String::from("7")]
        );

        assert_eq!(
            store.removed_levels,
            HashSet::from([
                String::from("3"),
                String::from("4"),
                String::from("5"),
                String::from("8"),
                String::from("9"),
                String::from("10"),
            ])
        );

        assert_eq!(statistics.number_of_working_levels, 10);

        assert_eq!(
            *trading_engine.closed_orders_by_take_profit.borrow(),
            vec![String::from("12"), String::from("15")]
        );

        assert_eq!(
            *trading_engine.closed_orders_by_stop_loss.borrow(),
            vec![String::from("13"), String::from("16")]
        );

        assert_eq!(
            *chart_traces_modifier
                .number_of_take_profit_entities
                .borrow(),
            4
        );
        assert_eq!(
            *chart_traces_modifier.number_of_stop_loss_entities.borrow(),
            4
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_orders_backtesting__optimization_and_no_trading_mode__should_not_place_orders_and_add_entities_to_chart_traces(
    ) {
        // Notation
        // o — crossed open price
        // s — exceed amount of candles in small corridor
        // b — exceed amount of candles in big corridor
        // p — price is beyond stop loss

        // Working level indexes
        // 2(buy), 7(sell)  — o && !s && !b && !p
        // 3(buy), 8(sell)  — !o
        // 4(buy), 9(sell)  — o && s
        // 5(buy), 10(sell) — o && !s && b

        let current_tick = BasicTickProperties {
            bid: dec!(1.30000),
            ..Default::default()
        };

        let current_candle = StepBacktestingCandleProperties::default();

        let params = TestParams::default();

        let mut store = TestStore::new(testing_orders(), testing_working_levels());

        let mut config = StepBacktestingConfig::default(50);
        let mut statistics = StepBacktestingStatistics {
            number_of_working_levels: 16,
            ..Default::default()
        };

        let stores = UpdateOrdersBacktestingStores {
            main: &mut store,
            config: &mut config,
            statistics: &mut statistics,
        };

        let trading_engine = TestTradingEngine::default();
        let chart_traces_modifier = TestChartTracesModifier::default();
        let level_conditions = TestLevelConditions::default();

        let utils = UpdateOrdersBacktestingUtils {
            trading_engine: &trading_engine,
            chart_traces_modifier: &chart_traces_modifier,
            level_conditions: &level_conditions,
        };

        let no_trading_mode = true;

        env::set_var("MODE", "optimization");

        let order_utils = OrderUtilsImpl::new();

        order_utils
            .update_orders_backtesting(
                &current_tick,
                &current_candle,
                &params,
                stores,
                utils,
                no_trading_mode,
            )
            .unwrap();

        assert_eq!(
            *level_conditions
                .level_exceeds_amount_of_candles_small_corridor_number_of_calls
                .borrow(),
            8
        );
        assert_eq!(statistics.deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing, 2);

        assert_eq!(
            *level_conditions
                .level_exceeds_amount_of_candles_big_corridor_number_of_calls
                .borrow(),
            6
        );
        assert_eq!(
            statistics
                .deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing,
            2
        );

        assert_eq!(
            *level_conditions
                .price_is_beyond_stop_loss_number_of_calls
                .borrow(),
            4
        );
        assert_eq!(statistics.deleted_by_price_being_beyond_stop_loss, 2);

        assert!(trading_engine.opened_orders.borrow().is_empty());

        assert_eq!(
            store.removed_levels,
            HashSet::from([
                String::from("3"),
                String::from("4"),
                String::from("5"),
                String::from("8"),
                String::from("9"),
                String::from("10"),
            ])
        );

        assert_eq!(statistics.number_of_working_levels, 10);

        assert_eq!(
            *trading_engine.closed_orders_by_take_profit.borrow(),
            vec![String::from("12"), String::from("15")]
        );

        assert_eq!(
            *trading_engine.closed_orders_by_stop_loss.borrow(),
            vec![String::from("13"), String::from("16")]
        );

        assert_eq!(
            *chart_traces_modifier
                .number_of_take_profit_entities
                .borrow(),
            0
        );
        assert_eq!(
            *chart_traces_modifier.number_of_stop_loss_entities.borrow(),
            0
        );
    }
}
