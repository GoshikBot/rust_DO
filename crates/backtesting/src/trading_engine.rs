use crate::{BacktestingTradingEngineConfig, ClosePositionBy, OpenPositionBy, Units};
use base::entities::order::{
    BasicOrderProperties, OrderId, OrderPrice, OrderStatus, OrderType, OrderVolume,
};
use base::entities::{Item, LOT};

use anyhow::Result;
use base::stores::order_store::BasicOrderStore;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn open_position<O>(
    order: Item<OrderId, O>,
    by: OpenPositionBy,
    order_store: &mut impl BasicOrderStore,
    trading_config: &mut BacktestingTradingEngineConfig,
) -> Result<()>
where
    O: Into<BasicOrderProperties>,
{
    let order_props = order.props.into();

    if order_props.status != OrderStatus::Pending {
        anyhow::bail!("order status is not pending: {:?}", order_props);
    }

    let price = match by {
        OpenPositionBy::OpenPrice => order_props.prices.open,
        OpenPositionBy::CurrentTickPrice(current_tick_price) => current_tick_price,
    };

    match order_props.r#type {
        OrderType::Buy => buy_instrument(price, order_props.volume, trading_config),
        OrderType::Sell => sell_instrument(price, order_props.volume, trading_config),
    }

    order_store.update_order_status(&order.id, OrderStatus::Opened)
}

pub fn close_position<O>(
    order: Item<OrderId, O>,
    by: ClosePositionBy,
    order_store: &mut impl BasicOrderStore<OrderProperties = O>,
    trading_config: &mut BacktestingTradingEngineConfig,
) -> Result<()>
where
    O: Into<BasicOrderProperties>,
{
    let order_props = order.props.into();

    if order_props.status != OrderStatus::Opened {
        anyhow::bail!("order status is not opened: {:?}", order_props);
    }

    let price = match by {
        ClosePositionBy::TakeProfit => order_props.prices.take_profit,
        ClosePositionBy::StopLoss => order_props.prices.stop_loss,
        ClosePositionBy::CurrentTickPrice(current_tick_price) => current_tick_price,
    };

    match order_props.r#type {
        OrderType::Buy => sell_instrument(price, order_props.volume, trading_config),
        OrderType::Sell => buy_instrument(price, order_props.volume, trading_config),
    }

    order_store.update_order_status(&order.id, OrderStatus::Closed)?;

    let order_statuses: Vec<_> = order_store
        .get_all_orders()?
        .into_iter()
        .map(|order| order.props.into().status)
        .collect();

    if no_opened_orders(&order_statuses) {
        trading_config.balances.real = trading_config.balances.processing;
        if trading_config.balances.real <= dec!(0) {
            anyhow::bail!(
                "real balance is less than or equal to zero: {:?}",
                trading_config.balances.real
            );
        }
    }

    Ok(())
}

fn no_opened_orders(order_statuses: &[OrderStatus]) -> bool {
    order_statuses
        .iter()
        .all(|status| status != &OrderStatus::Opened)
}

/// Executes a buy market order.
fn buy_instrument(
    mut price: OrderPrice,
    volume: OrderVolume,
    trading_config: &mut BacktestingTradingEngineConfig,
) {
    if trading_config.use_spread {
        // ask price
        price += trading_config.spread / dec!(2);
    }

    let units = (volume * Decimal::from(LOT))
        .trunc()
        .to_string()
        .parse::<Units>()
        .unwrap();
    let trade_value = Decimal::from(units) * price;

    trading_config.balances.processing -= trade_value;

    trading_config.units += units;
    trading_config.trades += 1;
}

/// Executes a sell market order.
fn sell_instrument(
    mut price: OrderPrice,
    volume: OrderVolume,
    trading_config: &mut BacktestingTradingEngineConfig,
) {
    if trading_config.use_spread {
        // bid price
        price -= trading_config.spread / dec!(2);
    }

    let units = (volume * Decimal::from(LOT))
        .trunc()
        .to_string()
        .parse::<Units>()
        .unwrap();
    let trade_value = Decimal::from(units) * price;

    trading_config.balances.processing += trade_value;

    trading_config.units -= units;
    trading_config.trades += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BacktestingBalances;
    use base::entities::order::BasicOrderPrices;
    use std::collections::HashMap;

    #[derive(Default)]
    struct TestOrderStore {
        orders: HashMap<OrderId, Item<OrderId, BasicOrderProperties>>,
        id_counter: u32,
    }

    impl TestOrderStore {
        fn new() -> Self {
            Default::default()
        }

        fn create_order(&mut self, order_id: OrderId, props: BasicOrderProperties) {
            self.orders.insert(
                order_id.clone(),
                Item {
                    id: order_id,
                    props,
                },
            );
        }

        fn get_order_by_id(&self, id: &str) -> Option<Item<OrderId, BasicOrderProperties>> {
            self.orders.get(id).cloned()
        }
    }

    impl BasicOrderStore for TestOrderStore {
        type OrderProperties = BasicOrderProperties;

        fn create_order(&mut self, properties: Self::OrderProperties) -> Result<OrderId> {
            let order_id = self.id_counter.to_string();
            self.id_counter += 1;

            self.orders.insert(
                order_id.clone(),
                Item {
                    id: order_id.clone(),
                    props: properties,
                },
            );

            Ok(order_id)
        }

        fn get_order_by_id(
            &self,
            _id: &str,
        ) -> Result<Option<Item<OrderId, Self::OrderProperties>>> {
            unimplemented!()
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

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                status: OrderStatus::Opened,
                ..Default::default()
            },
        );

        order_store.create_order(
            String::from("2"),
            BasicOrderProperties {
                status: OrderStatus::Closed,
                ..Default::default()
            },
        );

        assert!(open_position(
            order_store.get_order_by_id("1").unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("order status is not pending"));

        assert!(open_position(
            order_store.get_order_by_id("2").unwrap(),
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

        order_store.create_order(
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
        );

        open_position(
            order_store.get_order_by_id("1").unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

        assert_eq!(updated_order.props.status, OrderStatus::Opened);
        assert_eq!(trading_config.balances.processing, dec!(5856.13));
        assert_eq!(trading_config.units, 3000);
        assert_eq!(trading_config.trades, 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn open_position__buy_order_by_current_tick_price_with_spread__should_successfully_open_position(
    ) {
        let mut trading_config = BacktestingTradingEngineConfig::default();
        let mut order_store = TestOrderStore::new();

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                ..Default::default()
            },
        );

        let current_tick_price = dec!(1.20586);

        open_position(
            order_store.get_order_by_id("1").unwrap(),
            OpenPositionBy::CurrentTickPrice(current_tick_price),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                ..Default::default()
            },
        );

        let current_tick_price = dec!(1.20586);

        open_position(
            order_store.get_order_by_id("1").unwrap(),
            OpenPositionBy::CurrentTickPrice(current_tick_price),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
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
        );

        open_position(
            order_store.get_order_by_id("1").unwrap(),
            OpenPositionBy::OpenPrice,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

        assert_eq!(updated_order.props.status, OrderStatus::Opened);
        assert_eq!(trading_config.balances.processing, dec!(14_143.57));
        assert_eq!(trading_config.units, -3000);
        assert_eq!(trading_config.trades, 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn open_position__sell_order_by_current_tick_price_with_spread__should_successfully_open_position(
    ) {
        let mut trading_config = BacktestingTradingEngineConfig::default();
        let mut order_store = TestOrderStore::new();

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                ..Default::default()
            },
        );

        let current_tick_price = dec!(1.20586);

        open_position(
            order_store.get_order_by_id("1").unwrap(),
            OpenPositionBy::CurrentTickPrice(current_tick_price),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                status: OrderStatus::Pending,
                ..Default::default()
            },
        );

        order_store.create_order(
            String::from("2"),
            BasicOrderProperties {
                status: OrderStatus::Closed,
                ..Default::default()
            },
        );

        assert!(close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap_err()
        .to_string()
        .contains("order status is not opened"));

        assert!(close_position(
            order_store.get_order_by_id("2").unwrap(),
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

        order_store.create_order(
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
        );

        assert!(close_position(
            order_store.get_order_by_id("1").unwrap(),
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

        order_store.create_order(
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
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
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
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::StopLoss,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

        assert_eq!(updated_order.props.status, OrderStatus::Closed);
        assert_eq!(trading_config.balances.processing, dec!(14_143.57));
        assert_eq!(trading_config.balances.real, dec!(14_143.57));
        assert_eq!(trading_config.units, -3000);
        assert_eq!(trading_config.trades, 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn close_position__buy_order_by_take_profit_without_spread__should_successfully_close_position()
    {
        let mut trading_config = BacktestingTradingEngineConfig {
            use_spread: false,
            ..Default::default()
        };
        let mut order_store = TestOrderStore::new();

        order_store.create_order(
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
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

        assert_eq!(updated_order.props.status, OrderStatus::Closed);
        assert_eq!(trading_config.balances.processing, dec!(14_143.72));
        assert_eq!(trading_config.balances.real, dec!(14_143.72));
        assert_eq!(trading_config.units, -3000);
        assert_eq!(trading_config.trades, 1);
    }

    #[test]
    #[allow(non_snake_case)]
    fn close_position__buy_order_by_current_tick_price_with_spread__should_successfully_close_position(
    ) {
        let mut trading_config = BacktestingTradingEngineConfig::default();
        let mut order_store = TestOrderStore::new();

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::CurrentTickPrice(dec!(1.38124)),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
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
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::TakeProfit,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
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
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::StopLoss,
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::CurrentTickPrice(dec!(1.38124)),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

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

        order_store.create_order(
            String::from("1"),
            BasicOrderProperties {
                r#type: OrderType::Sell,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        );

        order_store.create_order(
            String::from("2"),
            BasicOrderProperties {
                r#type: OrderType::Buy,
                volume: dec!(0.03),
                status: OrderStatus::Opened,
                ..Default::default()
            },
        );

        close_position(
            order_store.get_order_by_id("1").unwrap(),
            ClosePositionBy::CurrentTickPrice(dec!(1.38124)),
            &mut order_store,
            &mut trading_config,
        )
        .unwrap();

        let updated_order = order_store.get_order_by_id("1").unwrap();

        assert_eq!(updated_order.props.status, OrderStatus::Closed);
        assert_eq!(trading_config.balances.processing, dec!(5_856.13));
        assert_eq!(trading_config.balances.real, dec!(10_000));
        assert_eq!(trading_config.units, 3000);
        assert_eq!(trading_config.trades, 1);
    }
}
