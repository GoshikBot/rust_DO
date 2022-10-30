use super::*;
use crate::{trading_engine, BacktestingBalances};
use base::entities::order::BasicOrderPrices;
use std::collections::HashMap;

#[derive(Default)]
struct TestOrderStore {
    orders: HashMap<OrderId, Item<OrderId, BasicOrderProperties>>,
}

impl TestOrderStore {
    fn new() -> Self {
        Default::default()
    }
}

impl BasicOrderStore for TestOrderStore {
    type OrderProperties = BasicOrderProperties;

    fn create_order(
        &mut self,
        id: OrderId,
        properties: Self::OrderProperties,
    ) -> Result<Item<OrderId, Self::OrderProperties>> {
        let new_order = Item {
            id: id.clone(),
            props: properties,
        };

        self.orders.insert(id, new_order.clone());

        Ok(new_order)
    }

    fn get_order_by_id(&self, id: &str) -> Result<Option<Item<OrderId, Self::OrderProperties>>> {
        Ok(self.orders.get(id).cloned())
    }

    fn get_all_orders(&self) -> Result<Vec<Item<OrderId, Self::OrderProperties>>> {
        Ok(self.orders.values().cloned().collect())
    }

    fn update_order_status(&mut self, order_id: &str, new_status: OrderStatus) -> Result<()> {
        self.orders.get_mut(order_id).unwrap().props.status = new_status;

        Ok(())
    }
}

#[test]
#[allow(non_snake_case)]
fn open_position__order_status_is_different_from_pending__should_return_error() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                status: OrderStatus::Opened,
                ..Default::default()
            },
        )
        .unwrap();

    order_store
        .create_order(
            String::from("2"),
            BasicOrderProperties {
                status: OrderStatus::Closed,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(trading_engine
        .open_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("order status is not pending"));

    assert!(trading_engine
        .open_position(
            &order_store.get_order_by_id("2").unwrap().unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("order status is not pending"));
}

#[test]
#[allow(non_snake_case)]
fn open_position__buy_order_by_open_price_with_spread__should_successfully_open_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                prices: BasicOrderPrices {
                    open: dec!(1.38124),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    trading_engine
        .open_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Opened);
    assert_eq!(trading_config.balances.processing, dec!(5856.13));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn open_position__buy_order_by_current_tick_price_with_spread__should_successfully_open_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                ..Default::default()
            },
        )
        .unwrap();

    let current_tick_price = dec!(1.20586);

    trading_engine
        .open_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            OpenPositionBy::CurrentTickPrice(current_tick_price),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Opened);
    assert_eq!(trading_config.balances.processing, dec!(6382.27));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn open_position__buy_order_by_current_tick_price_without_spread__should_successfully_open_position(
) {
    let mut trading_config = BacktestingTradingEngineConfig {
        use_spread: false,
        ..Default::default()
    };

    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                ..Default::default()
            },
        )
        .unwrap();

    let current_tick_price = dec!(1.20586);

    trading_engine
        .open_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            OpenPositionBy::CurrentTickPrice(current_tick_price),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Opened);
    assert_eq!(trading_config.balances.processing, dec!(6382.42));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn open_position__sell_order_by_open_price_with_spread__should_successfully_open_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                prices: BasicOrderPrices {
                    open: dec!(1.38124),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

    trading_engine
        .open_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Opened);
    assert_eq!(trading_config.balances.processing, dec!(14_143.57));
    assert_eq!(trading_config.units, -3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn open_position__sell_order_by_current_tick_price_with_spread__should_successfully_open_position()
{
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                ..Default::default()
            },
        )
        .unwrap();

    let current_tick_price = dec!(1.20586);

    trading_engine
        .open_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            OpenPositionBy::CurrentTickPrice(current_tick_price),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Opened);
    assert_eq!(trading_config.balances.processing, dec!(13_617.43));
    assert_eq!(trading_config.units, -3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__order_status_is_different_from_opened__should_return_error() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                status: OrderStatus::Pending,
                ..Default::default()
            },
        )
        .unwrap();

    order_store
        .create_order(
            String::from("2"),
            BasicOrderProperties {
                status: OrderStatus::Closed,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("order status is not opened"));

    assert!(trading_engine
        .close_position(
            &order_store.get_order_by_id("2").unwrap().unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("order status is not opened"));
}

#[test]
#[allow(non_snake_case)]
fn close_position__all_positions_become_closed_with_zero_balance__should_return_error() {
    let mut trading_config = BacktestingTradingEngineConfig {
        balances: BacktestingBalances::new(dec!(4_143.87)),
        ..Default::default()
    };

    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                prices: BasicOrderPrices {
                    stop_loss: dec!(1.38124),
                    ..Default::default()
                },
            },
        )
        .unwrap();

    assert!(trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::StopLoss,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("real balance is less than or equal to zero: 0"))
}

#[test]
#[allow(non_snake_case)]
fn close_position__buy_order_by_take_profit_with_spread__should_successfully_close_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                prices: BasicOrderPrices {
                    take_profit: dec!(1.38124),
                    ..Default::default()
                },
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(14_143.57));
    assert_eq!(trading_config.balances.real, dec!(14_143.57));
    assert_eq!(trading_config.units, -3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__buy_order_by_stop_loss_with_spread__should_successfully_close_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                prices: BasicOrderPrices {
                    stop_loss: dec!(1.38124),
                    ..Default::default()
                },
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::StopLoss,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(14_143.57));
    assert_eq!(trading_config.balances.real, dec!(14_143.57));
    assert_eq!(trading_config.units, -3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__buy_order_by_take_profit_without_spread__should_successfully_close_position() {
    let mut trading_config = BacktestingTradingEngineConfig {
        use_spread: false,
        ..Default::default()
    };
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                prices: BasicOrderPrices {
                    take_profit: dec!(1.38124),
                    ..Default::default()
                },
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(14_143.72));
    assert_eq!(trading_config.balances.real, dec!(14_143.72));
    assert_eq!(trading_config.units, -3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__buy_order_by_current_tick_price_with_spread__should_successfully_close_position()
{
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::CurrentTickPrice(dec!(1.38124)),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(14_143.57));
    assert_eq!(trading_config.balances.real, dec!(14_143.57));
    assert_eq!(trading_config.units, -3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__sell_order_by_take_profit_with_spread__should_successfully_close_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                prices: BasicOrderPrices {
                    take_profit: dec!(1.38124),
                    ..Default::default()
                },
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(5_856.13));
    assert_eq!(trading_config.balances.real, dec!(5_856.13));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__sell_order_by_stop_loss_with_spread__should_successfully_close_position() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                prices: BasicOrderPrices {
                    stop_loss: dec!(1.38124),
                    ..Default::default()
                },
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::StopLoss,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(5_856.13));
    assert_eq!(trading_config.balances.real, dec!(5_856.13));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__sell_order_by_current_tick_price_with_spread__should_successfully_close_position(
) {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::CurrentTickPrice(dec!(1.38124)),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(5_856.13));
    assert_eq!(trading_config.balances.real, dec!(5_856.13));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}

#[test]
#[allow(non_snake_case)]
fn close_position__there_are_still_opened_orders__should_not_update_real_balance() {
    let mut trading_config = BacktestingTradingEngineConfig::default();
    let mut order_store = TestOrderStore::new();
    let trading_engine = BacktestingTradingEngine::new();

    order_store
        .create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        )
        .unwrap();

    order_store
        .create_order(
            String::from("2"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        )
        .unwrap();

    trading_engine
        .close_position(
            &order_store.get_order_by_id("1").unwrap().unwrap(),
            ClosePositionBy::CurrentTickPrice(dec!(1.38124)),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

    let updated_order = order_store.get_order_by_id("1").unwrap().unwrap();

    assert_eq!(updated_order.props.status, OrderStatus::Closed);
    assert_eq!(trading_config.balances.processing, dec!(5_856.13));
    assert_eq!(trading_config.balances.real, dec!(10_000));
    assert_eq!(trading_config.units, 3000);
    assert_eq!(trading_config.trades, 1);
}
