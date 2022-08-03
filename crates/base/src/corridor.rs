use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::entities::candle::BasicCandleProperties;
use crate::entities::CandleMainProperties;
use crate::helpers::{points_to_price, price_to_points};
use crate::params::{ParamValue, StrategyParams};

/// Candle can be the corridor leader if its size is less or equal to the current volatility.
pub fn candle_can_be_corridor_leader(candle_properties: &CandleMainProperties) -> bool {
    if candle_properties.size <= candle_properties.volatility.into() {
        return true;
    }

    false
}

/// Checks if a candle is inside a corridor basing on a leading candle.
pub fn is_in_corridor(
    candle: &BasicCandleProperties,
    leading_candle: &BasicCandleProperties,
    max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
) -> bool {
    diff_between_edges(candle.edge_prices.high, Edge::High, leading_candle)
        <= max_distance_from_corridor_leading_candle_pins_pct
        && diff_between_edges(candle.edge_prices.low, Edge::Low, leading_candle)
            <= max_distance_from_corridor_leading_candle_pins_pct
}

#[derive(Debug, Copy, Clone)]
enum Edge {
    High,
    Low,
}

type ComparisonPrice = Decimal;
type Difference = Decimal;

/// Calculates a difference between the passed price and the corridor leading candle's edge
/// in % of the corridor leading candle size.
fn diff_between_edges(
    price: ComparisonPrice,
    edge: Edge,
    leading_candle: &BasicCandleProperties,
) -> Difference {
    match edge {
        Edge::High => {
            (price - leading_candle.edge_prices.high)
                / points_to_price(leading_candle.main_props.size)
                * dec!(100)
        }
        Edge::Low => {
            (leading_candle.edge_prices.low - price)
                / points_to_price(leading_candle.main_props.size)
                * dec!(100.0)
        }
    }
}

/// Shifts the corridor leader by one from the beginning of the corridor and tries to find
/// the appropriate leader for the new candle. The corridor will be cropped
/// to the closest appropriate leader.
pub fn crop_corridor_to_closest_leader(
    corridor: &[BasicCandleProperties],
    new_candle: &BasicCandleProperties,
    max_distance_from_corridor_leading_candle_pins_pct: ParamValue,
    candle_can_be_corridor_leader: impl Fn(&CandleMainProperties) -> bool,
    is_in_corridor: impl Fn(&BasicCandleProperties, &BasicCandleProperties, ParamValue) -> bool,
) -> Option<Vec<BasicCandleProperties>> {
    for (i, candle) in corridor.iter().enumerate() {
        if candle_can_be_corridor_leader(&candle.main_props)
            && is_in_corridor(
                new_candle,
                candle,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{CandlePrices, CandleType};
    use chrono::Utc;
    use std::cell::RefCell;

    #[test]
    #[allow(non_snake_case)]
    fn candle_can_be_corridor_leader__true() {
        let candle_properties = CandleMainProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(150),
            volatility: 160,
        };

        assert!(candle_can_be_corridor_leader(&candle_properties));
    }

    #[test]
    #[allow(non_snake_case)]
    fn candle_can_be_corridor_leader__false() {
        let candle_properties = CandleMainProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: dec!(180),
            volatility: 160,
        };

        assert!(!candle_can_be_corridor_leader(&candle_properties));
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_in_corridor__candle_is_in_range_of_leading_candle__true() {
        let current_candle = BasicCandleProperties {
            main_props: CandleMainProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: dec!(399),
                volatility: 271,
            },
            edge_prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22999),
                low: dec!(1.22600),
                close: dec!(1.22857),
            },
        };

        let leading_candle = BasicCandleProperties {
            main_props: CandleMainProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: dec!(288.0),
                volatility: 271,
            },
            edge_prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22943),
                low: dec!(1.22655),
                close: dec!(1.22857),
            },
        };

        assert!(is_in_corridor(&current_candle, &leading_candle, dec!(20)));
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_in_corridor__candle_is_beyond_the_range_of_leading_candle__false() {
        let current_candle = BasicCandleProperties {
            main_props: CandleMainProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: dec!(404.0),
                volatility: 271,
            },
            edge_prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.23001),
                low: dec!(1.22597),
                close: dec!(1.22857),
            },
        };

        let leading_candle = BasicCandleProperties {
            main_props: CandleMainProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: dec!(288.0),
                volatility: 271,
            },
            edge_prices: CandlePrices {
                open: dec!(1.22664),
                high: dec!(1.22943),
                low: dec!(1.22655),
                close: dec!(1.22857),
            },
        };

        assert!(!is_in_corridor(&current_candle, &leading_candle, dec!(20)));
    }

    #[test]
    #[allow(non_snake_case)]
    fn crop_corridor_to_closest_leader__third_candle_is_appropriate_leader__new_existing_corridor()
    {
        let current_corridor = [
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.0),
                    high: dec!(0.0),
                    low: dec!(0.0),
                    close: dec!(0.0),
                },
            },
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.1),
                    high: dec!(0.1),
                    low: dec!(0.1),
                    close: dec!(0.1),
                },
            },
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.2),
                    high: dec!(0.2),
                    low: dec!(0.2),
                    close: dec!(0.2),
                },
            },
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.3),
                    high: dec!(0.3),
                    low: dec!(0.3),
                    close: dec!(0.3),
                },
            },
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.4),
                    high: dec!(0.4),
                    low: dec!(0.4),
                    close: dec!(0.4),
                },
            },
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.5),
                    high: dec!(0.5),
                    low: dec!(0.5),
                    close: dec!(0.5),
                },
            },
            BasicCandleProperties {
                main_props: Default::default(),
                edge_prices: CandlePrices {
                    open: dec!(0.6),
                    high: dec!(0.6),
                    low: dec!(0.6),
                    close: dec!(0.6),
                },
            },
        ];

        let new_candle = BasicCandleProperties {
            main_props: Default::default(),
            edge_prices: CandlePrices {
                open: dec!(0.7),
                high: dec!(0.7),
                low: dec!(0.7),
                close: dec!(0.7),
            },
        };
        let max_distance_from_corridor_leading_candle_pins_pct = dec!(20);

        let number_of_calls_to_candle_can_be_corridor_leader = RefCell::new(0);
        let candle_can_be_corridor_leader = |_candle_properties: &CandleMainProperties| {
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

        let new_corridor = crop_corridor_to_closest_leader(
            &current_corridor,
            &new_candle,
            max_distance_from_corridor_leading_candle_pins_pct,
            candle_can_be_corridor_leader,
            is_in_corridor,
        )
        .unwrap();

        let mut new_expected_corridor = (&current_corridor[2..]).to_vec();
        new_expected_corridor.push(new_candle);

        assert_eq!(new_corridor, new_expected_corridor);
    }
}
