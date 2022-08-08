use anyhow::{bail, Result};
use backtesting::Balance;
use base::entities::order::{BasicOrderPrices, BasicOrderProperties, OrderType, OrderVolume};
use base::entities::{PRICE_DECIMAL_PLACES, TARGET_LOGGER_ENV, VOLUME_DECIMAL_PLACES};
use base::{
    entities::{candle::CandleVolatility, Item, LOT},
    helpers::points_to_price,
    params::StrategyParams,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::entities::{
    order::StepOrderProperties,
    params::{StepPointParam, StepRatioParam},
    working_levels::{BasicWLProperties, WLId},
};

/// Creates the chain of orders from the particular level when this level is crossed.
pub fn get_new_chain_of_orders<W>(
    level: &Item<WLId, W>,
    params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    current_volatility: CandleVolatility,
    current_balance: Balance,
) -> Result<Vec<StepOrderProperties>>
where
    W: Into<BasicWLProperties> + Clone,
{
    let level: Item<WLId, BasicWLProperties> = Item {
        id: level.id.clone(),
        props: level.props.clone().into(),
    };

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

    let volume_per_order = get_volume_per_order(params, distance_between_orders, current_balance)?;

    let (mut price_for_current_order, stop_loss) = match level.props.r#type {
        OrderType::Buy => {
            let price_for_current_order = level.props.price - distance_from_level_to_first_order;
            let stop_loss = level.props.price - distance_from_level_to_stop_loss;
            (
                price_for_current_order.round_dp(PRICE_DECIMAL_PLACES),
                stop_loss.round_dp(PRICE_DECIMAL_PLACES),
            )
        }
        OrderType::Sell => {
            let price_for_current_order = level.props.price + distance_from_level_to_first_order;
            let stop_loss = level.props.price + distance_from_level_to_stop_loss;
            (
                price_for_current_order.round_dp(PRICE_DECIMAL_PLACES),
                stop_loss.round_dp(PRICE_DECIMAL_PLACES),
            )
        }
    };

    let take_profit = level.props.price.round_dp(PRICE_DECIMAL_PLACES);

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
                r#type: level.props.r#type,
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

        match level.props.r#type {
            OrderType::Buy => price_for_current_order -= distance_between_orders,
            OrderType::Sell => price_for_current_order += distance_between_orders,
        }

        price_for_current_order = price_for_current_order.round_dp(PRICE_DECIMAL_PLACES);
    }

    Ok(chain_of_orders)
}

type MaxLossPerChainOfOrders = Decimal;

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

type DistanceBetweenOrders = Decimal;

/// Calculates the volume per order based on the max loss per the chain of orders.
fn get_volume_per_order(
    params: &impl StrategyParams<PointParam = StepPointParam, RatioParam = StepRatioParam>,
    distance_between_orders: DistanceBetweenOrders,
    current_balance: Balance,
) -> Result<OrderVolume> {
    let max_loss = get_max_loss_per_chain_of_orders_in_price(params, current_balance)?;

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

// pub fn update_orders_backtesting(
//     current_tick: &BasicTickProperties,
//     current_candle: &BasicCandleProperties,
//     config: &mut StepBacktestingConfig,
//     order_store: &impl BasicOrderStore<OrderProperties = >,
// ) -> Result<()> {
//     for order in working_level_store.get_all_orders()? {
//         if order.props.main_props.status == OrderStatus::Opened && current {}
//     }
//
//     todo!()
// }

#[cfg(test)]
mod tests {
    use base::params::ParamValue;
    use chrono::Utc;
    use rust_decimal_macros::dec;

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

        let chain_of_orders =
            get_new_chain_of_orders(&level, &params, volatility, balance).unwrap();

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

        let chain_of_orders = get_new_chain_of_orders(&level, &params, volatility, balance);

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

        let chain_of_orders = get_new_chain_of_orders(&level, &params, volatility, balance);

        assert!(chain_of_orders.is_err());
    }
}
