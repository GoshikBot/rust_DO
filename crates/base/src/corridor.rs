use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::entities::candle::{BasicCandleProperties, CandleId};
use crate::entities::Item;
use crate::helpers::points_to_price;
use crate::params::ParamValue;

#[derive(Debug, Copy, Clone)]
enum Edge {
    High,
    Low,
}

type ComparisonPrice = Decimal;
type Difference = Decimal;

pub trait BasicCorridorUtils {
    /// Candle can be the corridor leader if its size is less or equal to the current volatility.
    fn candle_can_be_corridor_leader(candle_properties: &impl AsRef<BasicCandleProperties>)
        -> bool;

    /// Checks if a candle is inside a corridor basing on a leading candle.
    fn candle_is_in_corridor<C>(
        candle: &C,
        leading_candle: &C,
        max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
    ) -> bool
    where
        C: AsRef<BasicCandleProperties>;

    /// Shifts the corridor leader by one from the beginning of the corridor and tries to find
    /// the appropriate leader for the new candle. The corridor will be cropped
    /// to the closest appropriate leader.
    fn crop_corridor_to_closest_leader<C>(
        corridor: &[Item<CandleId, C>],
        new_candle: &Item<CandleId, C>,
        max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
        candle_can_be_corridor_leader: &dyn Fn(&C) -> bool,
        is_in_corridor: &dyn Fn(&C, &C, ParamValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>
    where
        C: AsRef<BasicCandleProperties> + Clone;
}

pub struct BasicCorridorUtilsImpl;

impl BasicCorridorUtilsImpl {
    /// Calculates a difference between the passed price and the corridor leading candle's edge
    /// in % of the corridor leading candle size.
    fn diff_between_edges(
        price: ComparisonPrice,
        edge: Edge,
        leading_candle: &BasicCandleProperties,
    ) -> Difference {
        match edge {
            Edge::High => {
                (price - leading_candle.prices.high) / points_to_price(leading_candle.size)
                    * dec!(100)
            }
            Edge::Low => {
                (leading_candle.prices.low - price) / points_to_price(leading_candle.size)
                    * dec!(100.0)
            }
        }
    }
}

impl BasicCorridorUtils for BasicCorridorUtilsImpl {
    fn candle_can_be_corridor_leader(
        candle_properties: &impl AsRef<BasicCandleProperties>,
    ) -> bool {
        candle_properties.as_ref().size <= candle_properties.as_ref().volatility.into()
    }

    fn candle_is_in_corridor<C>(
        candle: &C,
        leading_candle: &C,
        max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
    ) -> bool
    where
        C: AsRef<BasicCandleProperties>,
    {
        let candle = candle.as_ref();
        let leading_candle = leading_candle.as_ref();

        Self::diff_between_edges(candle.prices.high, Edge::High, leading_candle)
            <= max_distance_from_corridor_leading_candle_pins_pct
            && Self::diff_between_edges(candle.prices.low, Edge::Low, leading_candle)
                <= max_distance_from_corridor_leading_candle_pins_pct
    }

    fn crop_corridor_to_closest_leader<C>(
        corridor: &[Item<CandleId, C>],
        new_candle: &Item<CandleId, C>,
        max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
        candle_can_be_corridor_leader: &dyn Fn(&C) -> bool,
        is_in_corridor: &dyn Fn(&C, &C, ParamValue) -> bool,
    ) -> Option<Vec<Item<CandleId, C>>>
    where
        C: AsRef<BasicCandleProperties> + Clone,
    {
        for (i, candle) in corridor.iter().enumerate() {
            if candle_can_be_corridor_leader(&candle.props)
                && is_in_corridor(
                    &new_candle.props,
                    &candle.props,
                    max_distance_from_corridor_leading_candle_pins_pct,
                )
            {
                let mut new_corridor = corridor[i..].to_vec();
                new_corridor.push(new_candle.clone());
                return Some(new_corridor);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{BasicTickProperties, CandlePrices, CandleType};
    use chrono::Utc;
    use std::cell::RefCell;

    #[test]
    #[allow(non_snake_case)]
    fn candle_can_be_corridor_leader__appropriate_candle__should_return_true() {
        let candle_properties = BasicCandleProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(150),
            volatility: 160,
            ..Default::default()
        };

        assert!(BasicCorridorUtilsImpl::candle_can_be_corridor_leader(
            &candle_properties
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn candle_can_be_corridor_leader__inappropriate_candle__should_return_false() {
        let candle_properties = BasicCandleProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(180),
            volatility: 160,
            ..Default::default()
        };

        assert!(!BasicCorridorUtilsImpl::candle_can_be_corridor_leader(
            &candle_properties,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_in_corridor__candle_is_in_range_of_leading_candle__true() {
        let current_candle = BasicCandleProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(399),
            volatility: 271,
            prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22999),
                low: dec!(1.22600),
                close: dec!(1.22857),
            },
        };

        let leading_candle = BasicCandleProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(288.0),
            volatility: 271,
            prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22943),
                low: dec!(1.22655),
                close: dec!(1.22857),
            },
        };

        assert!(BasicCorridorUtilsImpl::candle_is_in_corridor(
            &current_candle,
            &leading_candle,
            dec!(20)
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_in_corridor__candle_is_beyond_the_range_of_leading_candle__false() {
        let current_candle = BasicCandleProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(404.0),
            volatility: 271,
            prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.23001),
                low: dec!(1.22597),
                close: dec!(1.22857),
            },
        };

        let leading_candle = BasicCandleProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(288.0),
            volatility: 271,
            prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22943),
                low: dec!(1.22655),
                close: dec!(1.22857),
            },
        };

        assert!(!BasicCorridorUtilsImpl::candle_is_in_corridor(
            &current_candle,
            &leading_candle,
            dec!(20)
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn crop_corridor_to_closest_leader__third_candle_is_appropriate_leader__new_existing_corridor()
    {
        let current_corridor = [
            Item {
                id: String::from("1"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.0),
                        high: dec!(0.0),
                        low: dec!(0.0),
                        close: dec!(0.0),
                    },
                    ..Default::default()
                },
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.1),
                        high: dec!(0.1),
                        low: dec!(0.1),
                        close: dec!(0.1),
                    },
                    ..Default::default()
                },
            },
            Item {
                id: String::from("3"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.2),
                        high: dec!(0.2),
                        low: dec!(0.2),
                        close: dec!(0.2),
                    },
                    ..Default::default()
                },
            },
            Item {
                id: String::from("4"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.3),
                        high: dec!(0.3),
                        low: dec!(0.3),
                        close: dec!(0.3),
                    },
                    ..Default::default()
                },
            },
            Item {
                id: String::from("5"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.4),
                        high: dec!(0.4),
                        low: dec!(0.4),
                        close: dec!(0.4),
                    },
                    ..Default::default()
                },
            },
            Item {
                id: String::from("6"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.5),
                        high: dec!(0.5),
                        low: dec!(0.5),
                        close: dec!(0.5),
                    },
                    ..Default::default()
                },
            },
            Item {
                id: String::from("7"),
                props: BasicCandleProperties {
                    prices: CandlePrices {
                        open: dec!(0.6),
                        high: dec!(0.6),
                        low: dec!(0.6),
                        close: dec!(0.6),
                    },
                    ..Default::default()
                },
            },
        ];

        let new_candle = Item {
            id: String::from("8"),
            props: BasicCandleProperties {
                prices: CandlePrices {
                    open: dec!(0.7),
                    high: dec!(0.7),
                    low: dec!(0.7),
                    close: dec!(0.7),
                },
                ..Default::default()
            },
        };
        let max_distance_from_corridor_leading_candle_pins_pct = dec!(20);

        let number_of_calls_to_candle_can_be_corridor_leader = RefCell::new(0);
        let candle_can_be_corridor_leader = |_candle_properties: &BasicCandleProperties| {
            *number_of_calls_to_candle_can_be_corridor_leader.borrow_mut() += 1;
            *number_of_calls_to_candle_can_be_corridor_leader.borrow() > 1
        };

        let number_of_calls_to_is_in_corridor = RefCell::new(0);
        let is_in_corridor =
            |_candle: &BasicCandleProperties,
             _leading_candle: &BasicCandleProperties,
             _max_distance_from_corridor_leading_candle_pins_pct: ParamValue| {
                *number_of_calls_to_is_in_corridor.borrow_mut() += 1;
                *number_of_calls_to_is_in_corridor.borrow() > 1
            };

        let new_corridor = BasicCorridorUtilsImpl::crop_corridor_to_closest_leader(
            &current_corridor,
            &new_candle,
            max_distance_from_corridor_leading_candle_pins_pct,
            &candle_can_be_corridor_leader,
            &is_in_corridor,
        )
        .unwrap();

        let mut new_expected_corridor = (&current_corridor[2..]).to_vec();
        new_expected_corridor.push(new_candle);

        assert_eq!(new_corridor, new_expected_corridor);
    }
}
