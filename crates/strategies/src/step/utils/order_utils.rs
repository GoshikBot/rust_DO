use crate::step::utils::backtesting_charts::{
    ChartIndex, ChartTraceEntity, StepBacktestingChartTraces,
};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use crate::step::utils::entities::working_levels::{
    BacktestingWLProperties, CorridorType, WLStatus,
};
use crate::step::utils::entities::{Mode, MODE_ENV};
use crate::step::utils::level_conditions::{LevelConditions, MinAmountOfCandles};
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use crate::step::utils::stores::{StepBacktestingConfig, StepBacktestingStatistics};
use anyhow::{bail, Result};
use backtesting::trading_engine::TradingEngine;
use backtesting::{BacktestingTradingEngineConfig, Balance, ClosePositionBy, OpenPositionBy};
use base::entities::order::{
    BasicOrderPrices, BasicOrderProperties, OrderPrice, OrderStatus, OrderType, OrderVolume,
};
use base::entities::tick::{HistoricalTickPrice, TickPrice, UniversalTickPrice};
use base::entities::{
    BasicTickProperties, CANDLE_PRICE_DECIMAL_PLACES, SIGNIFICANT_DECIMAL_PLACES,
};
use base::stores::order_store::BasicOrderStore;
use base::{
    entities::{candle::CandleVolatility, Item, LOT},
    helpers::points_to_price,
    params::StrategyParams,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::marker::PhantomData;
use std::str::FromStr;

use super::entities::{
    order::StepOrderProperties,
    params::{StepPointParam, StepRatioParam},
    working_levels::{BasicWLProperties, WLId},
};

pub trait OrderUtils {
    /// Creates the chain of orders from the particular level when this level is crossed.
    fn get_new_chain_of_orders<W>(
        level: &Item<WLId, W>,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        current_volatility: CandleVolatility,
        current_balance: Balance,
    ) -> Result<Vec<StepOrderProperties>>
    where
        W: AsRef<BasicWLProperties>;

    /// Places and closed orders.
    fn update_orders_backtesting<TrEng, C, R, W, P, A>(
        current_tick: &BasicTickProperties<HistoricalTickPrice>,
        current_candle: &StepBacktestingCandleProperties,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        stores: UpdateOrdersBacktestingStores<W>,
        utils: UpdateOrdersBacktestingUtils<TrEng, C, R, W, P, A>,
        no_trading_mode: bool,
    ) -> Result<()>
    where
        W: BasicOrderStore<OrderProperties = StepOrderProperties>
            + StepWorkingLevelStore<
                WorkingLevelProperties = BacktestingWLProperties,
                OrderProperties = StepOrderProperties,
            >,
        TrEng: TradingEngine,
        C: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
        R: Fn(&str, &W, CorridorType, MinAmountOfCandles) -> Result<bool>,
        P: Fn(UniversalTickPrice, OrderPrice, OrderType) -> bool,
        A: Fn(&[StepOrderProperties]) -> bool;

    fn close_all_orders_backtesting<S>(
        current_tick_price: HistoricalTickPrice,
        current_candle_chart_index: ChartIndex,
        store: &mut S,
        config: &mut StepBacktestingConfig,
        trading_engine: &impl TradingEngine,
        add_entity_to_chart_traces: &impl Fn(
            ChartTraceEntity,
            &mut StepBacktestingChartTraces,
            ChartIndex,
        ),
    ) -> Result<()>
    where
        S: StepWorkingLevelStore<
                WorkingLevelProperties = BacktestingWLProperties,
                OrderProperties = StepOrderProperties,
            > + BasicOrderStore<OrderProperties = StepOrderProperties>;
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

        let max_loss_per_chain_of_orders_pct = (current_balance
            * params.get_point_param_value(StepPointParam::MaxLossPerOneChainOfOrdersPctOfBalance)
            / dec!(100))
        .round_dp(SIGNIFICANT_DECIMAL_PLACES);

        log::debug!(
            "max loss per chain of orders in price is {}; current balance — {}",
            max_loss_per_chain_of_orders_pct,
            current_balance
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

        log::debug!("volume per order — {}", volume_per_order);

        Ok(volume_per_order.round_dp(SIGNIFICANT_DECIMAL_PLACES))
    }
}

impl OrderUtils for OrderUtilsImpl {
    fn get_new_chain_of_orders<W>(
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
                    price_for_current_order.round_dp(CANDLE_PRICE_DECIMAL_PLACES),
                    stop_loss.round_dp(CANDLE_PRICE_DECIMAL_PLACES),
                )
            }
            OrderType::Sell => {
                let price_for_current_order =
                    level.props.as_ref().price + distance_from_level_to_first_order;
                let stop_loss = level.props.as_ref().price + distance_from_level_to_stop_loss;
                (
                    price_for_current_order.round_dp(CANDLE_PRICE_DECIMAL_PLACES),
                    stop_loss.round_dp(CANDLE_PRICE_DECIMAL_PLACES),
                )
            }
        };

        let take_profit = level
            .props
            .as_ref()
            .price
            .round_dp(CANDLE_PRICE_DECIMAL_PLACES);

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

            price_for_current_order = price_for_current_order.round_dp(CANDLE_PRICE_DECIMAL_PLACES);
        }

        Ok(chain_of_orders)
    }

    fn update_orders_backtesting<TrEng, C, R, W, P, A>(
        current_tick: &BasicTickProperties<HistoricalTickPrice>,
        current_candle: &StepBacktestingCandleProperties,
        params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
        stores: UpdateOrdersBacktestingStores<W>,
        utils: UpdateOrdersBacktestingUtils<TrEng, C, R, W, P, A>,
        no_trading_mode: bool,
    ) -> Result<()>
    where
        W: BasicOrderStore<OrderProperties = StepOrderProperties>
            + StepWorkingLevelStore<
                WorkingLevelProperties = BacktestingWLProperties,
                OrderProperties = StepOrderProperties,
            >,
        TrEng: TradingEngine,
        C: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
        R: Fn(&str, &W, CorridorType, MinAmountOfCandles) -> Result<bool>,
        P: Fn(UniversalTickPrice, OrderPrice, OrderType) -> bool,
        A: Fn(&[StepOrderProperties]) -> bool,
    {
        'level: for level in stores.main.get_all_working_levels()? {
            for order in stores.main.get_working_level_chain_of_orders(&level.id)? {
                match order.props.base.status {
                    OrderStatus::Pending => {
                        if (order.props.base.r#type == OrderType::Buy
                            && current_tick.bid.low <= order.props.base.prices.open)
                            || (order.props.base.r#type == OrderType::Sell
                                && current_tick.bid.high >= order.props.base.prices.open)
                        {
                            let mut remove_working_level = false;
                            let mut try_to_open_position = false;

                            if stores.main.get_working_level_status(&level.id)?.unwrap()
                                == WLStatus::Created
                            {
                                if !(utils.level_exceeds_amount_of_candles_in_corridor)(
                                    &order.props.working_level_id,
                                    stores.main,
                                    CorridorType::Small,
                                    params.get_point_param_value(StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel),
                                )? {
                                    if !(utils.level_exceeds_amount_of_candles_in_corridor)(
                                        &order.props.working_level_id,
                                        stores.main,
                                        CorridorType::Big,
                                        params.get_point_param_value(StepPointParam::MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel),
                                    )? {
                                        stores.main.move_working_level_to_active(&level.id)?;

                                        try_to_open_position = true;
                                    } else {
                                        stores.statistics.deleted_by_exceeding_amount_of_candles_in_big_corridor_before_activation_crossing += 1;
                                        remove_working_level = true;
                                    }
                                } else {
                                    stores.statistics.deleted_by_exceeding_amount_of_candles_in_small_corridor_before_activation_crossing += 1;
                                    remove_working_level = true;
                                }
                            } else {
                                try_to_open_position = true;
                            }

                            if try_to_open_position && !no_trading_mode {
                                let price_is_beyond_stop_loss = (utils.price_is_beyond_stop_loss)(
                                    UniversalTickPrice::Historical(current_tick.bid),
                                    order.props.base.prices.stop_loss,
                                    order.props.base.r#type,
                                );

                                let level_has_no_active_orders = (utils.level_has_no_active_orders)(
                                    &stores
                                        .main
                                        .get_working_level_chain_of_orders(&level.id)?
                                        .into_iter()
                                        .map(|o| o.props)
                                        .collect::<Vec<_>>(),
                                );

                                if price_is_beyond_stop_loss && level_has_no_active_orders {
                                    stores.statistics.deleted_by_price_being_beyond_stop_loss += 1;
                                    remove_working_level = true;
                                } else {
                                    utils.trading_engine.open_position(
                                        &order,
                                        OpenPositionBy::OpenPrice,
                                        stores.main,
                                        &mut stores.config.trading_engine,
                                    )?;

                                    // updated order after opening position for closing position to have actual data
                                    let order = stores.main.get_order_by_id(&order.id)?.unwrap();

                                    if price_is_beyond_stop_loss {
                                        utils.trading_engine.close_position(
                                            &order,
                                            ClosePositionBy::StopLoss,
                                            stores.main,
                                            &mut stores.config.trading_engine,
                                        )?;

                                        let working_level_chart_index = stores
                                            .main
                                            .get_working_level_by_id(&order.props.working_level_id)?
                                            .unwrap()
                                            .props
                                            .chart_index;

                                        if Mode::from_str(&dotenv::var(MODE_ENV).unwrap()).unwrap()
                                            != Mode::Optimization
                                        {
                                            (utils.add_entity_to_chart_traces)(
                                                ChartTraceEntity::TakeProfit {
                                                    take_profit_price: order
                                                        .props
                                                        .base
                                                        .prices
                                                        .take_profit,
                                                    working_level_chart_index,
                                                },
                                                &mut stores.config.chart_traces,
                                                current_candle.chart_index,
                                            );

                                            (utils.add_entity_to_chart_traces)(
                                                ChartTraceEntity::StopLoss {
                                                    stop_loss_price: order
                                                        .props
                                                        .base
                                                        .prices
                                                        .stop_loss,
                                                    working_level_chart_index,
                                                },
                                                &mut stores.config.chart_traces,
                                                current_candle.chart_index,
                                            );
                                        }
                                    }
                                }
                            }

                            if remove_working_level {
                                stores
                                    .main
                                    .remove_working_level(&order.props.working_level_id)?;

                                stores.statistics.number_of_working_levels -= 1;

                                continue 'level;
                            }
                        }
                    }
                    OrderStatus::Opened => {
                        let mut add_to_chart_traces = false;

                        if (order.props.base.r#type == OrderType::Buy
                            && current_tick.bid.high >= order.props.base.prices.take_profit)
                            || (order.props.base.r#type == OrderType::Sell
                                && current_tick.bid.low <= order.props.base.prices.take_profit)
                        {
                            add_to_chart_traces = true;
                            utils.trading_engine.close_position(
                                &order,
                                ClosePositionBy::TakeProfit,
                                stores.main,
                                &mut stores.config.trading_engine,
                            )?;
                        } else if (order.props.base.r#type == OrderType::Buy
                            && current_tick.bid.low <= order.props.base.prices.stop_loss)
                            || (order.props.base.r#type == OrderType::Sell
                                && current_tick.bid.high >= order.props.base.prices.stop_loss)
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
                            && Mode::from_str(&dotenv::var(MODE_ENV).unwrap()).unwrap()
                                != Mode::Optimization
                        {
                            (utils.add_entity_to_chart_traces)(
                                ChartTraceEntity::TakeProfit {
                                    take_profit_price: order.props.base.prices.take_profit,
                                    working_level_chart_index,
                                },
                                &mut stores.config.chart_traces,
                                current_candle.chart_index,
                            );

                            (utils.add_entity_to_chart_traces)(
                                ChartTraceEntity::StopLoss {
                                    stop_loss_price: order.props.base.prices.stop_loss,
                                    working_level_chart_index,
                                },
                                &mut stores.config.chart_traces,
                                current_candle.chart_index,
                            );
                        }
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn close_all_orders_backtesting<S>(
        current_tick_price: HistoricalTickPrice,
        current_candle_chart_index: ChartIndex,
        store: &mut S,
        config: &mut StepBacktestingConfig,
        trading_engine: &impl TradingEngine,
        add_entity_to_chart_traces: &impl Fn(
            ChartTraceEntity,
            &mut StepBacktestingChartTraces,
            ChartIndex,
        ),
    ) -> Result<()>
    where
        S: StepWorkingLevelStore<
                WorkingLevelProperties = BacktestingWLProperties,
                OrderProperties = StepOrderProperties,
            > + BasicOrderStore<OrderProperties = StepOrderProperties>,
    {
        for level in store.get_active_working_levels()? {
            for order in store
                .get_working_level_chain_of_orders(&level.id)?
                .into_iter()
                .filter(|o| o.props.base.status == OrderStatus::Opened)
            {
                trading_engine.close_position(
                    &order,
                    ClosePositionBy::CurrentTickPrice(current_tick_price.close),
                    store,
                    &mut config.trading_engine,
                )?;

                add_entity_to_chart_traces(
                    ChartTraceEntity::ClosePrice {
                        working_level_chart_index: level.props.chart_index,
                        close_price: current_tick_price.close,
                    },
                    &mut config.chart_traces,
                    current_candle_chart_index,
                );

                add_entity_to_chart_traces(
                    ChartTraceEntity::TakeProfit {
                        take_profit_price: order.props.base.prices.take_profit,
                        working_level_chart_index: level.props.chart_index,
                    },
                    &mut config.chart_traces,
                    current_candle_chart_index,
                );

                add_entity_to_chart_traces(
                    ChartTraceEntity::StopLoss {
                        stop_loss_price: order.props.base.prices.stop_loss,
                        working_level_chart_index: level.props.chart_index,
                    },
                    &mut config.chart_traces,
                    current_candle_chart_index,
                );
            }
        }

        Ok(())
    }
}

type MaxLossPerChainOfOrders = Decimal;

type DistanceBetweenOrders = Decimal;

pub struct UpdateOrdersBacktestingUtils<'a, E, C, R, W, P, A>
where
    E: TradingEngine,
    C: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    W: StepWorkingLevelStore,
    R: Fn(&str, &W, CorridorType, MinAmountOfCandles) -> Result<bool>,
    P: Fn(UniversalTickPrice, OrderPrice, OrderType) -> bool,
    A: Fn(&[StepOrderProperties]) -> bool,
{
    pub add_entity_to_chart_traces: &'a C,
    pub level_exceeds_amount_of_candles_in_corridor: &'a R,
    pub price_is_beyond_stop_loss: &'a P,
    pub level_has_no_active_orders: &'a A,
    pub trading_engine: &'a E,
    phantom: PhantomData<W>,
}

impl<'a, E, C, R, W, P, A> UpdateOrdersBacktestingUtils<'a, E, C, R, W, P, A>
where
    E: TradingEngine,
    C: Fn(ChartTraceEntity, &mut StepBacktestingChartTraces, ChartIndex),
    W: StepWorkingLevelStore,
    R: Fn(&str, &W, CorridorType, MinAmountOfCandles) -> Result<bool>,
    P: Fn(UniversalTickPrice, OrderPrice, OrderType) -> bool,
    A: Fn(&[StepOrderProperties]) -> bool,
{
    pub fn new(
        trading_engine: &'a E,
        add_entity_to_chart_traces: &'a C,
        level_exceeds_amount_of_candles_in_corridor: &'a R,
        price_is_beyond_stop_loss: &'a P,
        level_has_no_active_orders: &'a A,
    ) -> Self {
        Self {
            trading_engine,
            add_entity_to_chart_traces,
            level_exceeds_amount_of_candles_in_corridor,
            price_is_beyond_stop_loss,
            level_has_no_active_orders,
            phantom: PhantomData,
        }
    }
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
mod tests;
