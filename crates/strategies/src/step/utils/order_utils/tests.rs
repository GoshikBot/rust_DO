use crate::step::utils::backtesting_charts::StepBacktestingChartTraces;
use crate::step::utils::entities::working_levels::{
    LevelTime, WLMaxCrossingValue, WLPrice, WLStatus,
};
use crate::step::utils::level_conditions::MinAmountOfCandles;
use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use backtesting::BacktestingTradingEngineConfig;
use base::entities::candle::CandleId;
use base::entities::order::{OrderId, OrderPrice};
use base::entities::tick::{TickPrice, TickTime};
use base::helpers::{Holiday, NumberOfDaysToExclude};
use base::params::ParamOutputValue;
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

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamOutputValue {
        match name {
            StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct => unreachable!(),
            StepPointParam::AmountOfOrders => dec!(5.0),
            StepPointParam::LevelExpirationDays => unreachable!(),
            StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel => {
                unreachable!()
            }
            StepPointParam::MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel => {
                unreachable!()
            }
            StepPointParam::MinAmountOfCandlesInCorridorDefiningEdgeBargaining => unreachable!(),
            StepPointParam::MaxLossPerOneChainOfOrdersPctOfBalance => dec!(10.0),
        }
    }

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        volatility: CandleVolatility,
    ) -> ParamOutputValue {
        let value = match name {
            StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles => unreachable!(),
            StepRatioParam::MinDistanceBetweenCurrentMaxAndMinAnglesForNewInnerAngleToAppear => unreachable!(),
            StepRatioParam::MinBreakDistance => unreachable!(),
            StepRatioParam::DistanceFromLevelToFirstOrder => dec!(0.7),
            StepRatioParam::DistanceFromLevelToStopLoss => dec!(3.6),
            StepRatioParam::DistanceFromLevelForSignalingOfMovingTakeProfits => unreachable!(),
            StepRatioParam::DistanceToMoveTakeProfits => unreachable!(),
            StepRatioParam::DistanceFromLevelForItsDeletion => unreachable!(),
            StepRatioParam::DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel => unreachable!(),
            StepRatioParam::DistanceDefiningNearbyLevelsOfTheSameType => unreachable!(),
            StepRatioParam::MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion => unreachable!(),
            StepRatioParam::RangeOfBigCorridorNearLevel => unreachable!(),
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

    let chain_of_orders =
        OrderUtilsImpl::get_new_chain_of_orders(&level, &params, volatility, balance).unwrap();

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

    let chain_of_orders =
        OrderUtilsImpl::get_new_chain_of_orders(&level, &params, volatility, balance);

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

    let chain_of_orders =
        OrderUtilsImpl::get_new_chain_of_orders(&level, &params, volatility, balance);

    assert!(chain_of_orders.is_err());
}

#[derive(Default)]
struct TestParams;

impl StrategyParams for TestParams {
    type PointParam = StepPointParam;
    type RatioParam = StepRatioParam;

    fn get_point_param_value(&self, _name: Self::PointParam) -> ParamOutputValue {
        dec!(1)
    }

    fn get_ratio_param_value(
        &self,
        _name: Self::RatioParam,
        _volatility: CandleVolatility,
    ) -> ParamOutputValue {
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

// update_orders_backtesting cases to test:
// - pending order (id: 1) && buy level (id: 1) && created && exceeds amount of candles
//   in small corridor (should remove level)
// - pending order (id: 2) && buy level (id: 2) && created && does NOT exceed amount of candles
//   in small corridor && exceeds amount of candles in big corridor (should remove level)
// - pending order (id: 3) && buy level (id: 3) && created && does NOT exceed amount of candles
//   in small corridor && does NOT exceed amount of candles in big corridor && trading is allowed
//   && price is beyond stop loss && level has NO active orders (should remove level)
// - pending order (id: 1 separate test) && buy level (id: 1 separate test) && created
//   && does NOT exceed amount of candles in small corridor && does NOT exceed amount of candles
//   in big corridor && trading is NOT allowed (should do nothing)
// - pending order (id: 4) && buy level (id: 4) && created && does NOT exceed amount of candles
//   in small corridor && does NOT exceed amount of candles in big corridor && trading is allowed
//   && price is beyond stop loss && level has active orders (should open position and immediately
//   close it by stop loss)
// - pending order (id: 5) && buy level (id: 4) && created && does NOT exceed amount of candles
//   in small corridor && does NOT exceed amount of candles in big corridor && trading is allowed
//   && price is NOT beyond stop loss && level has active orders (should open position)
// - pending order (id: 6) && buy level (id: 4) && created && order has NOT crossed open price
//   && does NOT exceed amount of candles in small corridor && does NOT exceed amount of candles
//   in big corridor && trading is allowed && price is NOT beyond stop loss && level has active
//   orders (should NOT open position)
// - pending order (id: 7) && buy level (id: 5) && active && trading is allowed && price
//   is NOT beyond stop loss && level has active orders (should open position)
// - open order (id: 8) && buy level (id: 5) && tick price crossed take profit
//   (should close position by take profit)
// - open order (id: 9) && buy level (id: 5) && tick price crossed stop loss
//   (should close position by stop loss)
//
// - pending order (id: 10) && sell level (id: 6) && created && exceeds amount of candles
//   in small corridor (should remove level)
// - pending order (id: 11) && sell level (id: 7) && created && does NOT exceed amount of candles
//   in small corridor && exceeds amount of candles in big corridor (should remove level)
// - pending order (id: 12) && sell level (id: 8) && created && does NOT exceed amount of candles
//   in small corridor && does NOT exceed amount of candles in big corridor && trading is allowed
//   && price is beyond stop loss && level has NO active orders (should remove level)
// - pending order (id: 2 separate test) && sell level (id: 2 separate test) && created
//   && does NOT exceed amount of candles in small corridor && does NOT exceed amount of candles
//   in big corridor && trading is NOT allowed (should do nothing)
// - pending order (id: 13) && sell level (id: 9) && created && does NOT exceed amount of candles
//   in small corridor && does NOT exceed amount of candles in big corridor && trading is allowed
//   && price is beyond stop loss && level has active orders (should open position and immediately
//   close it by stop loss)
// - pending order (id: 14) && sell level (id: 9) && created && does NOT exceed amount of candles
//   in small corridor && does NOT exceed amount of candles in big corridor && trading is allowed
//   && price is NOT beyond stop loss && level has active orders (should open position)
// - pending order (id: 15) && sell level (id: 9) && created && order has NOT crossed open price
//   && does NOT exceed amount of candles in small corridor && does NOT exceed amount of candles
//   in big corridor && trading is allowed && price is NOT beyond stop loss && level has active
//   orders (should NOT open position)
// - pending order (id: 16) && sell level (id: 10) && active && trading is allowed && price
//   is NOT beyond stop loss && level has active orders (should open position)
// - open order (id: 17) && sell level (id: 10) && tick price crossed take profit
//   (should close position by take profit)
// - open order (id: 18) && sell level (id: 10) && tick price crossed stop loss
//   (should close position by stop loss)

fn testing_working_levels() -> Vec<Item<WLId, BacktestingWLProperties>> {
    vec![
        Item {
            id: String::from("1"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                chart_index: 1,
            },
        },
        Item {
            id: String::from("2"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                chart_index: 2,
            },
        },
        Item {
            id: String::from("3"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                chart_index: 3,
            },
        },
        Item {
            id: String::from("4"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                chart_index: 4,
            },
        },
        Item {
            id: String::from("5"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                chart_index: 5,
            },
        },
        Item {
            id: String::from("6"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                chart_index: 6,
            },
        },
        Item {
            id: String::from("7"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                chart_index: 7,
            },
        },
        Item {
            id: String::from("8"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                chart_index: 8,
            },
        },
        Item {
            id: String::from("9"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                chart_index: 9,
            },
        },
        Item {
            id: String::from("10"),
            props: BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                chart_index: 10,
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
                        open: dec!(1.28000),
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
                        open: dec!(1.28000),
                        stop_loss: dec!(1.88888),
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
                        open: dec!(1.28000),
                        stop_loss: dec!(1.88888),
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
                        open: dec!(1.28000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "4".to_string(),
            },
        },
        Item {
            id: String::from("6"),
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
                working_level_id: "4".to_string(),
            },
        },
        Item {
            id: String::from("7"),
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
                working_level_id: "5".to_string(),
            },
        },
        Item {
            id: String::from("8"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    status: OrderStatus::Opened,
                    prices: BasicOrderPrices {
                        take_profit: dec!(1.26000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "5".to_string(),
            },
        },
        Item {
            id: String::from("9"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    status: OrderStatus::Opened,
                    prices: BasicOrderPrices {
                        stop_loss: dec!(1.27500),
                        take_profit: dec!(1.28000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "5".to_string(),
            },
        },
        Item {
            id: String::from("10"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "6".to_string(),
            },
        },
        Item {
            id: String::from("11"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "7".to_string(),
            },
        },
        Item {
            id: String::from("12"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        stop_loss: dec!(1.88888),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "8".to_string(),
            },
        },
        Item {
            id: String::from("13"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        stop_loss: dec!(1.88888),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "9".to_string(),
            },
        },
        Item {
            id: String::from("14"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "9".to_string(),
            },
        },
        Item {
            id: String::from("15"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "9".to_string(),
            },
        },
        Item {
            id: String::from("16"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Pending,
                    prices: BasicOrderPrices {
                        open: dec!(1.26500),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "10".to_string(),
            },
        },
        Item {
            id: String::from("17"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Opened,
                    prices: BasicOrderPrices {
                        take_profit: dec!(1.27500),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "10".to_string(),
            },
        },
        Item {
            id: String::from("18"),
            props: StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    status: OrderStatus::Opened,
                    prices: BasicOrderPrices {
                        stop_loss: dec!(1.26500),
                        take_profit: dec!(1.24000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: "10".to_string(),
            },
        },
    ]
}

#[test]
#[allow(non_snake_case)]
fn update_orders_backtesting__debug_and_allow_trading_mode__should_successfully_place_and_close_particular_orders(
) {
    let current_tick = BasicTickProperties {
        bid: HistoricalTickPrice {
            low: dec!(1.27000),
            high: dec!(1.27000),
            close: dec!(1.27000),
        },
        ..Default::default()
    };

    let current_candle = StepBacktestingCandleProperties::default();

    let params = TestParams::default();

    let mut store = InMemoryStepBacktestingStore::default();

    for level in testing_working_levels() {
        let created_level = store.create_working_level(level.id, level.props).unwrap();

        if matches!(created_level.id.as_str(), "5" | "10") {
            store
                .move_working_level_to_active(&created_level.id)
                .unwrap();
        }
    }

    for order in testing_orders() {
        store.create_order(order.id, order.props).unwrap();
    }

    let mut config = StepBacktestingConfig::default(50);
    let mut statistics = StepBacktestingStatistics {
        number_of_working_levels: 10,
        ..Default::default()
    };

    let stores = UpdateOrdersBacktestingStores {
        main: &mut store,
        config: &mut config,
        statistics: &mut statistics,
    };

    let trading_engine = TestTradingEngine::default();

    let level_exceeds_amount_of_candles_small_corridor_number_of_calls = RefCell::new(0);
    let level_exceeds_amount_of_candles_big_corridor_number_of_calls = RefCell::new(0);

    let level_exceeds_amount_of_candles_in_corridor =
        |level_id: &str,
         _working_level_store: &InMemoryStepBacktestingStore,
         corridor_type: CorridorType,
         _min_amount_of_candles: MinAmountOfCandles| {
            match corridor_type {
                CorridorType::Small => {
                    *level_exceeds_amount_of_candles_small_corridor_number_of_calls.borrow_mut() +=
                        1
                }
                CorridorType::Big => {
                    *level_exceeds_amount_of_candles_big_corridor_number_of_calls.borrow_mut() += 1
                }
            }

            match level_id {
                "2" | "3" | "4" | "7" | "8" | "9" if corridor_type == CorridorType::Small => {
                    Ok(false)
                }
                "3" | "4" | "8" | "9" if corridor_type == CorridorType::Big => Ok(false),
                _ => Ok(true),
            }
        };

    let price_is_beyond_stop_loss_number_of_calls = RefCell::new(0);

    let price_is_beyond_stop_loss =
        |_current_tick_price: UniversalTickPrice,
         stop_loss_price: OrderPrice,
         _working_level_type: OrderType| {
            *price_is_beyond_stop_loss_number_of_calls.borrow_mut() += 1;
            stop_loss_price == dec!(1.88888)
        };

    let number_of_stop_loss_entities = RefCell::new(0);
    let number_of_take_profit_entities = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_index: ChartIndex| {
            match entity {
                ChartTraceEntity::StopLoss { .. } => {
                    *number_of_stop_loss_entities.borrow_mut() += 1
                }
                ChartTraceEntity::TakeProfit { .. } => {
                    *number_of_take_profit_entities.borrow_mut() += 1
                }
                _ => unreachable!(),
            }
        };

    let level_has_no_active_orders = |orders: &[StepOrderProperties]| {
        for order in orders {
            match order.working_level_id.as_str() {
                "3" | "8" => return true,
                "4" | "5" | "9" | "10" => return false,
                _ => continue,
            }
        }

        false
    };

    let utils = UpdateOrdersBacktestingUtils::new(
        &trading_engine,
        &add_entity_to_chart_traces,
        &level_exceeds_amount_of_candles_in_corridor,
        &price_is_beyond_stop_loss,
        &level_has_no_active_orders,
    );

    let no_trading_mode = false;

    env::set_var("MODE", "debug");

    OrderUtilsImpl::update_orders_backtesting(
        &current_tick,
        &current_candle,
        &params,
        stores,
        utils,
        no_trading_mode,
    )
    .unwrap();

    assert_eq!(
        *level_exceeds_amount_of_candles_small_corridor_number_of_calls.borrow(),
        8
    );
    assert_eq!(
        statistics
            .deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing,
        2
    );

    assert_eq!(
        *level_exceeds_amount_of_candles_big_corridor_number_of_calls.borrow(),
        6
    );
    assert_eq!(
        statistics
            .deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing,
        2
    );

    assert_eq!(*price_is_beyond_stop_loss_number_of_calls.borrow(), 10);
    assert_eq!(statistics.deleted_by_price_being_beyond_stop_loss, 2);

    let opened_orders: HashSet<String> =
        HashSet::from_iter(trading_engine.opened_orders.borrow().iter().cloned());

    let expected_opened_orders = HashSet::from([
        String::from("4"),
        String::from("5"),
        String::from("7"),
        String::from("13"),
        String::from("14"),
        String::from("16"),
    ]);

    assert_eq!(
        opened_orders.intersection(&expected_opened_orders).count(),
        expected_opened_orders.len()
    );

    let active_working_levels = HashSet::from_iter(
        store
            .get_active_working_levels()
            .unwrap()
            .into_iter()
            .map(|level| level.id),
    );

    let expected_active_working_levels = HashSet::from([
        String::from("5"),
        String::from("10"),
        String::from("4"),
        String::from("9"),
    ]);

    assert_eq!(
        active_working_levels
            .intersection(&expected_active_working_levels)
            .count(),
        expected_active_working_levels.len()
    );

    assert_eq!(statistics.number_of_working_levels, 4);

    let closed_orders_by_take_profit: HashSet<String> = HashSet::from_iter(
        trading_engine
            .closed_orders_by_take_profit
            .borrow()
            .iter()
            .cloned(),
    );

    let expected_closed_orders_by_take_profit =
        HashSet::from([String::from("8"), String::from("17")]);

    assert_eq!(
        closed_orders_by_take_profit
            .intersection(&expected_closed_orders_by_take_profit)
            .count(),
        expected_closed_orders_by_take_profit.len()
    );

    let closed_orders_by_stop_loss: HashSet<String> = HashSet::from_iter(
        trading_engine
            .closed_orders_by_stop_loss
            .borrow()
            .iter()
            .cloned(),
    );

    let expected_closed_orders_by_stop_loss = HashSet::from([
        String::from("9"),
        String::from("18"),
        String::from("4"),
        String::from("13"),
    ]);

    assert_eq!(
        closed_orders_by_stop_loss
            .intersection(&expected_closed_orders_by_stop_loss)
            .count(),
        expected_closed_orders_by_stop_loss.len()
    );

    assert_eq!(*number_of_take_profit_entities.borrow(), 6);
    assert_eq!(*number_of_stop_loss_entities.borrow(), 6);
}

#[test]
#[allow(non_snake_case)]
fn update_orders_backtesting__optimization_and_allow_trading_mode__should_successfully_place_and_close_particular_orders_buy_not_add_entities_to_chart_traces(
) {
    let current_tick = BasicTickProperties {
        bid: HistoricalTickPrice {
            low: dec!(1.27000),
            high: dec!(1.27000),
            close: dec!(1.27000),
        },
        ..Default::default()
    };

    let current_candle = StepBacktestingCandleProperties::default();

    let params = TestParams::default();

    let mut store = InMemoryStepBacktestingStore::default();

    for level in testing_working_levels() {
        let created_level = store.create_working_level(level.id, level.props).unwrap();

        if matches!(created_level.id.as_str(), "5" | "10") {
            store
                .move_working_level_to_active(&created_level.id)
                .unwrap();
        }
    }

    for order in testing_orders() {
        store.create_order(order.id, order.props).unwrap();
    }

    let mut config = StepBacktestingConfig::default(50);
    let mut statistics = StepBacktestingStatistics {
        number_of_working_levels: 10,
        ..Default::default()
    };

    let stores = UpdateOrdersBacktestingStores {
        main: &mut store,
        config: &mut config,
        statistics: &mut statistics,
    };

    let trading_engine = TestTradingEngine::default();

    let level_exceeds_amount_of_candles_small_corridor_number_of_calls = RefCell::new(0);
    let level_exceeds_amount_of_candles_big_corridor_number_of_calls = RefCell::new(0);

    let level_exceeds_amount_of_candles_in_corridor =
        |level_id: &str,
         _working_level_store: &InMemoryStepBacktestingStore,
         corridor_type: CorridorType,
         _min_amount_of_candles: MinAmountOfCandles| {
            match corridor_type {
                CorridorType::Small => {
                    *level_exceeds_amount_of_candles_small_corridor_number_of_calls.borrow_mut() +=
                        1
                }
                CorridorType::Big => {
                    *level_exceeds_amount_of_candles_big_corridor_number_of_calls.borrow_mut() += 1
                }
            }

            match level_id {
                "2" | "3" | "4" | "7" | "8" | "9" if corridor_type == CorridorType::Small => {
                    Ok(false)
                }
                "3" | "4" | "8" | "9" if corridor_type == CorridorType::Big => Ok(false),
                _ => Ok(true),
            }
        };

    let price_is_beyond_stop_loss_number_of_calls = RefCell::new(0);

    let price_is_beyond_stop_loss =
        |_current_tick_price: UniversalTickPrice,
         stop_loss_price: OrderPrice,
         _working_level_type: OrderType| {
            *price_is_beyond_stop_loss_number_of_calls.borrow_mut() += 1;
            stop_loss_price == dec!(1.88888)
        };

    let number_of_stop_loss_entities = RefCell::new(0);
    let number_of_take_profit_entities = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_index: ChartIndex| {
            match entity {
                ChartTraceEntity::StopLoss { .. } => {
                    *number_of_stop_loss_entities.borrow_mut() += 1
                }
                ChartTraceEntity::TakeProfit { .. } => {
                    *number_of_take_profit_entities.borrow_mut() += 1
                }
                _ => unreachable!(),
            }
        };

    let level_has_no_active_orders = |orders: &[StepOrderProperties]| {
        for order in orders {
            match order.working_level_id.as_str() {
                "3" | "8" => return true,
                "4" | "5" | "9" | "10" => return false,
                _ => continue,
            }
        }

        false
    };

    let utils = UpdateOrdersBacktestingUtils::new(
        &trading_engine,
        &add_entity_to_chart_traces,
        &level_exceeds_amount_of_candles_in_corridor,
        &price_is_beyond_stop_loss,
        &level_has_no_active_orders,
    );

    let no_trading_mode = false;

    env::set_var("MODE", "optimization");

    OrderUtilsImpl::update_orders_backtesting(
        &current_tick,
        &current_candle,
        &params,
        stores,
        utils,
        no_trading_mode,
    )
    .unwrap();

    assert_eq!(
        *level_exceeds_amount_of_candles_small_corridor_number_of_calls.borrow(),
        8
    );
    assert_eq!(
        statistics
            .deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing,
        2
    );

    assert_eq!(
        *level_exceeds_amount_of_candles_big_corridor_number_of_calls.borrow(),
        6
    );
    assert_eq!(
        statistics
            .deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing,
        2
    );

    assert_eq!(*price_is_beyond_stop_loss_number_of_calls.borrow(), 10);
    assert_eq!(statistics.deleted_by_price_being_beyond_stop_loss, 2);

    let opened_orders: HashSet<String> =
        HashSet::from_iter(trading_engine.opened_orders.borrow().iter().cloned());

    let expected_opened_orders = HashSet::from([
        String::from("4"),
        String::from("5"),
        String::from("7"),
        String::from("13"),
        String::from("14"),
        String::from("16"),
    ]);

    assert_eq!(
        opened_orders.intersection(&expected_opened_orders).count(),
        expected_opened_orders.len()
    );

    let active_working_levels = HashSet::from_iter(
        store
            .get_active_working_levels()
            .unwrap()
            .into_iter()
            .map(|level| level.id),
    );

    let expected_active_working_levels = HashSet::from([
        String::from("5"),
        String::from("10"),
        String::from("4"),
        String::from("9"),
    ]);

    assert_eq!(
        active_working_levels
            .intersection(&expected_active_working_levels)
            .count(),
        expected_active_working_levels.len()
    );

    assert_eq!(statistics.number_of_working_levels, 4);

    let closed_orders_by_take_profit: HashSet<String> = HashSet::from_iter(
        trading_engine
            .closed_orders_by_take_profit
            .borrow()
            .iter()
            .cloned(),
    );

    let expected_closed_orders_by_take_profit =
        HashSet::from([String::from("8"), String::from("17")]);

    assert_eq!(
        closed_orders_by_take_profit
            .intersection(&expected_closed_orders_by_take_profit)
            .count(),
        expected_closed_orders_by_take_profit.len()
    );

    let closed_orders_by_stop_loss: HashSet<String> = HashSet::from_iter(
        trading_engine
            .closed_orders_by_stop_loss
            .borrow()
            .iter()
            .cloned(),
    );

    let expected_closed_orders_by_stop_loss = HashSet::from([
        String::from("9"),
        String::from("18"),
        String::from("4"),
        String::from("13"),
    ]);

    assert_eq!(
        closed_orders_by_stop_loss
            .intersection(&expected_closed_orders_by_stop_loss)
            .count(),
        expected_closed_orders_by_stop_loss.len()
    );

    assert_eq!(*number_of_take_profit_entities.borrow(), 0);
    assert_eq!(*number_of_stop_loss_entities.borrow(), 0);
}

#[test]
#[allow(non_snake_case)]
fn update_orders_backtesting__no_trading_mode__should_not_place_orders() {
    let current_tick = BasicTickProperties {
        bid: HistoricalTickPrice {
            low: dec!(1.27000),
            high: dec!(1.27000),
            close: dec!(1.27000),
        },
        ..Default::default()
    };

    let current_candle = StepBacktestingCandleProperties::default();

    let params = TestParams::default();

    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            String::from("1"),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("1"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    prices: BasicOrderPrices {
                        open: dec!(1.28000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    store
        .create_working_level(
            String::from("2"),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Sell,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("2"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    prices: BasicOrderPrices {
                        open: dec!(1.27000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                working_level_id: String::from("2"),
            },
        )
        .unwrap();

    let mut config = StepBacktestingConfig::default(50);
    let mut statistics = StepBacktestingStatistics {
        number_of_working_levels: 2,
        ..Default::default()
    };

    let stores = UpdateOrdersBacktestingStores {
        main: &mut store,
        config: &mut config,
        statistics: &mut statistics,
    };

    let trading_engine = TestTradingEngine::default();

    let level_exceeds_amount_of_candles_small_corridor_number_of_calls = RefCell::new(0);
    let level_exceeds_amount_of_candles_big_corridor_number_of_calls = RefCell::new(0);

    let level_exceeds_amount_of_candles_in_corridor =
        |_level_id: &str,
         _working_level_store: &InMemoryStepBacktestingStore,
         corridor_type: CorridorType,
         _min_amount_of_candles: MinAmountOfCandles| {
            match corridor_type {
                CorridorType::Small => {
                    *level_exceeds_amount_of_candles_small_corridor_number_of_calls.borrow_mut() +=
                        1
                }
                CorridorType::Big => {
                    *level_exceeds_amount_of_candles_big_corridor_number_of_calls.borrow_mut() += 1
                }
            }

            Ok(false)
        };

    let price_is_beyond_stop_loss =
        |_current_tick_price: UniversalTickPrice,
         _stop_loss_price: OrderPrice,
         _working_level_type: OrderType| { false };

    let number_of_stop_loss_entities = RefCell::new(0);
    let number_of_take_profit_entities = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_index: ChartIndex| {
            match entity {
                ChartTraceEntity::StopLoss { .. } => {
                    *number_of_stop_loss_entities.borrow_mut() += 1
                }
                ChartTraceEntity::TakeProfit { .. } => {
                    *number_of_take_profit_entities.borrow_mut() += 1
                }
                _ => unreachable!(),
            }
        };

    let level_has_no_active_orders = |_orders: &[StepOrderProperties]| true;

    let utils = UpdateOrdersBacktestingUtils::new(
        &trading_engine,
        &add_entity_to_chart_traces,
        &level_exceeds_amount_of_candles_in_corridor,
        &price_is_beyond_stop_loss,
        &level_has_no_active_orders,
    );

    let no_trading_mode = true;

    env::set_var("MODE", "debug");

    OrderUtilsImpl::update_orders_backtesting(
        &current_tick,
        &current_candle,
        &params,
        stores,
        utils,
        no_trading_mode,
    )
    .unwrap();

    assert_eq!(
        *level_exceeds_amount_of_candles_small_corridor_number_of_calls.borrow(),
        2
    );
    assert_eq!(
        *level_exceeds_amount_of_candles_big_corridor_number_of_calls.borrow(),
        2
    );

    assert!(trading_engine.opened_orders.borrow().is_empty());
}

#[test]
#[allow(non_snake_case)]
fn update_orders_backtesting__tick_different_low_and_high_prices__should_properly_update_orders() {
    let current_tick = BasicTickProperties {
        bid: HistoricalTickPrice {
            low: dec!(1.27000),
            high: dec!(1.28000),
            close: dec!(1.27000),
        },
        ..Default::default()
    };

    let current_candle = StepBacktestingCandleProperties::default();

    let params = TestParams::default();

    let mut store = InMemoryStepBacktestingStore::default();

    store
        .create_working_level(
            String::from("1"),
            BacktestingWLProperties {
                base: BasicWLProperties {
                    r#type: OrderType::Buy,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("1"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    prices: BasicOrderPrices {
                        open: dec!(1.27500),
                        ..Default::default()
                    },
                    status: OrderStatus::Pending,
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("2"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    prices: BasicOrderPrices {
                        open: dec!(1.27500),
                        ..Default::default()
                    },
                    status: OrderStatus::Pending,
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("3"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    prices: BasicOrderPrices {
                        take_profit: dec!(1.27500),
                        ..Default::default()
                    },
                    status: OrderStatus::Opened,
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("4"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    prices: BasicOrderPrices {
                        take_profit: dec!(1.27500),
                        ..Default::default()
                    },
                    status: OrderStatus::Opened,
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("5"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Buy,
                    prices: BasicOrderPrices {
                        stop_loss: dec!(1.27500),
                        take_profit: dec!(1.40000),
                        ..Default::default()
                    },
                    status: OrderStatus::Opened,
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    store
        .create_order(
            String::from("6"),
            StepOrderProperties {
                base: BasicOrderProperties {
                    r#type: OrderType::Sell,
                    prices: BasicOrderPrices {
                        stop_loss: dec!(1.27500),
                        take_profit: dec!(1.25000),
                        ..Default::default()
                    },
                    status: OrderStatus::Opened,
                    ..Default::default()
                },
                working_level_id: String::from("1"),
            },
        )
        .unwrap();

    let mut config = StepBacktestingConfig::default(50);
    let mut statistics = StepBacktestingStatistics {
        number_of_working_levels: 2,
        ..Default::default()
    };

    let stores = UpdateOrdersBacktestingStores {
        main: &mut store,
        config: &mut config,
        statistics: &mut statistics,
    };

    let trading_engine = TestTradingEngine::default();

    let level_exceeds_amount_of_candles_number_of_calls = RefCell::new(0);

    let level_exceeds_amount_of_candles_in_corridor =
        |_level_id: &str,
         _working_level_store: &InMemoryStepBacktestingStore,
         corridor_type: CorridorType,
         _min_amount_of_candles: MinAmountOfCandles| {
            *level_exceeds_amount_of_candles_number_of_calls.borrow_mut() += 1;

            Ok(false)
        };

    let price_is_beyond_stop_loss =
        |_current_tick_price: UniversalTickPrice,
         _stop_loss_price: OrderPrice,
         _working_level_type: OrderType| { false };

    let number_of_stop_loss_entities = RefCell::new(0);
    let number_of_take_profit_entities = RefCell::new(0);

    let add_entity_to_chart_traces =
        |entity: ChartTraceEntity,
         _chart_traces: &mut StepBacktestingChartTraces,
         _current_candle_index: ChartIndex| {
            match entity {
                ChartTraceEntity::StopLoss { .. } => {
                    *number_of_stop_loss_entities.borrow_mut() += 1
                }
                ChartTraceEntity::TakeProfit { .. } => {
                    *number_of_take_profit_entities.borrow_mut() += 1
                }
                _ => unreachable!(),
            }
        };

    let level_has_no_active_orders = |_orders: &[StepOrderProperties]| true;

    let utils = UpdateOrdersBacktestingUtils::new(
        &trading_engine,
        &add_entity_to_chart_traces,
        &level_exceeds_amount_of_candles_in_corridor,
        &price_is_beyond_stop_loss,
        &level_has_no_active_orders,
    );

    let no_trading_mode = false;

    env::set_var("MODE", "debug");

    OrderUtilsImpl::update_orders_backtesting(
        &current_tick,
        &current_candle,
        &params,
        stores,
        utils,
        no_trading_mode,
    )
    .unwrap();

    assert_eq!(*level_exceeds_amount_of_candles_number_of_calls.borrow(), 2);

    assert_eq!(trading_engine.closed_orders_by_stop_loss.borrow().len(), 2);
    assert_eq!(
        trading_engine.closed_orders_by_take_profit.borrow().len(),
        2
    );
}
