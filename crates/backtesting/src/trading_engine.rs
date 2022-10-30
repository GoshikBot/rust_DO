use crate::{BacktestingTradingEngineConfig, ClosePositionBy, OpenPositionBy, Units};
use base::entities::order::{
    BasicOrderProperties, OrderId, OrderPrice, OrderStatus, OrderType, OrderVolume,
};
use base::entities::{Item, CANDLE_PRICE_DECIMAL_PLACES, LOT, SIGNIFICANT_DECIMAL_PLACES};
use std::fmt::Debug;

use anyhow::Result;
use base::stores::order_store::BasicOrderStore;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub trait TradingEngine {
    fn open_position<O>(
        &self,
        order: &Item<OrderId, O>,
        by: OpenPositionBy,
        order_store: &mut impl BasicOrderStore<OrderProperties = O>,
        trading_config: &mut BacktestingTradingEngineConfig,
    ) -> Result<()>
    where
        O: Into<BasicOrderProperties> + Clone + Debug;

    fn close_position<O>(
        &self,
        order: &Item<OrderId, O>,
        by: ClosePositionBy,
        order_store: &mut impl BasicOrderStore<OrderProperties = O>,
        trading_config: &mut BacktestingTradingEngineConfig,
    ) -> Result<()>
    where
        O: Into<BasicOrderProperties> + Clone + Debug;
}

#[derive(Default)]
pub struct BacktestingTradingEngine;

impl BacktestingTradingEngine {
    pub fn new() -> Self {
        Self::default()
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
    ) -> Result<()> {
        if trading_config.use_spread {
            // ask price
            price += trading_config.spread / dec!(2);
            price = price.round_dp(CANDLE_PRICE_DECIMAL_PLACES);
        }

        let units = (volume * Decimal::from(LOT))
            .trunc()
            .to_string()
            .parse::<Units>()?;

        let trade_value = (Decimal::from(units) * price).round_dp(SIGNIFICANT_DECIMAL_PLACES);

        trading_config.balances.processing -= trade_value;
        trading_config.balances.processing = trading_config
            .balances
            .processing
            .round_dp(SIGNIFICANT_DECIMAL_PLACES);

        trading_config.units += units;
        trading_config.trades += 1;

        Ok(())
    }

    /// Executes a sell market order.
    fn sell_instrument(
        mut price: OrderPrice,
        volume: OrderVolume,
        trading_config: &mut BacktestingTradingEngineConfig,
    ) -> Result<()> {
        if trading_config.use_spread {
            // bid price
            price -= trading_config.spread / dec!(2);
            price = price.round_dp(CANDLE_PRICE_DECIMAL_PLACES);
        }

        let units = (volume * Decimal::from(LOT))
            .trunc()
            .to_string()
            .parse::<Units>()?;

        let trade_value = (Decimal::from(units) * price).round_dp(SIGNIFICANT_DECIMAL_PLACES);

        trading_config.balances.processing += trade_value;
        trading_config.balances.processing = trading_config
            .balances
            .processing
            .round_dp(SIGNIFICANT_DECIMAL_PLACES);

        trading_config.units -= units;
        trading_config.trades += 1;

        Ok(())
    }
}

impl TradingEngine for BacktestingTradingEngine {
    fn open_position<O>(
        &self,
        order: &Item<OrderId, O>,
        by: OpenPositionBy,
        order_store: &mut impl BasicOrderStore<OrderProperties = O>,
        trading_config: &mut BacktestingTradingEngineConfig,
    ) -> Result<()>
    where
        O: Into<BasicOrderProperties> + Clone + Debug,
    {
        let order_props = order.props.clone().into();

        if order_props.status != OrderStatus::Pending {
            anyhow::bail!("order status is not pending: {:?}", order_props);
        }

        let price = match by {
            OpenPositionBy::OpenPrice => order_props.prices.open,
            OpenPositionBy::CurrentTickPrice(current_tick_price) => current_tick_price,
        };

        match order_props.r#type {
            OrderType::Buy => Self::buy_instrument(price, order_props.volume, trading_config)?,
            OrderType::Sell => Self::sell_instrument(price, order_props.volume, trading_config)?,
        }

        order_store.update_order_status(&order.id, OrderStatus::Opened)
    }

    fn close_position<O>(
        &self,
        order: &Item<OrderId, O>,
        by: ClosePositionBy,
        order_store: &mut impl BasicOrderStore<OrderProperties = O>,
        trading_config: &mut BacktestingTradingEngineConfig,
    ) -> Result<()>
    where
        O: Into<BasicOrderProperties> + Clone + Debug,
    {
        let order_props = order.props.clone().into();

        if order_props.status != OrderStatus::Opened {
            anyhow::bail!("order status is not opened: {:?}", order_props);
        }

        let price = match by {
            ClosePositionBy::TakeProfit => order_props.prices.take_profit,
            ClosePositionBy::StopLoss => order_props.prices.stop_loss,
            ClosePositionBy::CurrentTickPrice(current_tick_price) => current_tick_price,
        };

        match order_props.r#type {
            OrderType::Buy => Self::sell_instrument(price, order_props.volume, trading_config)?,
            OrderType::Sell => Self::buy_instrument(price, order_props.volume, trading_config)?,
        }

        order_store.update_order_status(&order.id, OrderStatus::Closed)?;

        let order_statuses: Vec<_> = order_store
            .get_all_orders()?
            .into_iter()
            .map(|order| order.props.into().status)
            .collect();

        if Self::no_opened_orders(&order_statuses) {
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
}

#[cfg(test)]
mod tests;
