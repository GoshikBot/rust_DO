use crate::entities::candle::BasicCandle;
use crate::entities::CandleBaseProperties;
use crate::helpers::{points_to_price, price_to_points};
use crate::params::{PointSettingValue, StrategyParams};

/// Candle can be the corridor leader if its size is less or equal to the current volatility.
pub fn candle_can_be_corridor_leader(candle_properties: &CandleBaseProperties) -> bool {
    if candle_properties.size <= candle_properties.volatility {
        return true;
    }

    false
}

/// Checks if a candle is inside a corridor basing on a leading candle.
pub fn is_in_corridor(
    candle: &BasicCandle,
    leading_candle: &BasicCandle,
    max_distance_from_corridor_leading_candle_pins_pct: PointSettingValue,
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

type ComparisonPrice = f32;
type Difference = f32;

/// Calculates a difference between the passed price and the corridor leading candle's edge
/// in % of the corridor leading candle size.
fn diff_between_edges(
    price: ComparisonPrice,
    edge: Edge,
    leading_candle: &BasicCandle,
) -> Difference {
    match edge {
        Edge::High => {
            (price - leading_candle.edge_prices.high)
                / points_to_price(leading_candle.properties.size)
                * 100.0
        }
        Edge::Low => {
            (leading_candle.edge_prices.low - price)
                / points_to_price(leading_candle.properties.size)
                * 100.0
        }
    }
}

/// Shifts the corridor leader by one from the beginning of the corridor and tries to find
/// the appropriate leader for the new candle. The corridor will be cropped
/// to the closest appropriate leader.
pub fn crop_corridor_to_closest_leader(
    corridor: &[BasicCandle],
    new_candle: &BasicCandle,
    max_distance_from_corridor_leading_candle_pins_pct: PointSettingValue,
    candle_can_be_corridor_leader: impl Fn(&CandleBaseProperties) -> bool,
    is_in_corridor: impl Fn(&BasicCandle, &BasicCandle, PointSettingValue) -> bool,
) -> Option<Vec<BasicCandle>> {
    for (i, candle) in corridor.iter().enumerate() {
        if candle_can_be_corridor_leader(&candle.properties)
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
    use crate::entities::{CandleEdgePrices, CandleType};
    use chrono::Utc;
    use std::cell::RefCell;

    #[test]
    #[allow(non_snake_case)]
    fn candle_can_be_corridor_leader__true() {
        let candle_properties = CandleBaseProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: 150.0,
            volatility: 160.0,
        };

        assert!(candle_can_be_corridor_leader(&candle_properties));
    }

    #[test]
    #[allow(non_snake_case)]
    fn candle_can_be_corridor_leader__false() {
        let candle_properties = CandleBaseProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: 180.0,
            volatility: 160.0,
        };

        assert!(!candle_can_be_corridor_leader(&candle_properties));
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_in_corridor__candle_is_in_range_of_leading_candle__true() {
        let current_candle = BasicCandle {
            properties: CandleBaseProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: 399.0,
                volatility: 271.0,
            },
            edge_prices: CandleEdgePrices {
                open: 1.22664,
                high: 1.22999,
                low: 1.22600,
                close: 1.22857,
            },
        };

        let leading_candle = BasicCandle {
            properties: CandleBaseProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: 288.0,
                volatility: 271.0,
            },
            edge_prices: CandleEdgePrices {
                open: 1.22664,
                high: 1.22943,
                low: 1.22655,
                close: 1.22857,
            },
        };

        assert!(is_in_corridor(&current_candle, &leading_candle, 20.0));
    }

    #[test]
    #[allow(non_snake_case)]
    fn is_in_corridor__candle_is_beyond_the_range_of_leading_candle__false() {
        let current_candle = BasicCandle {
            properties: CandleBaseProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: 404.0,
                volatility: 271.0,
            },
            edge_prices: CandleEdgePrices {
                open: 1.22664,
                high: 1.23001,
                low: 1.22597,
                close: 1.22857,
            },
        };

        let leading_candle = BasicCandle {
            properties: CandleBaseProperties {
                time: Utc::now().naive_utc(),
                r#type: CandleType::Green,
                size: 288.0,
                volatility: 271.0,
            },
            edge_prices: CandleEdgePrices {
                open: 1.22664,
                high: 1.22943,
                low: 1.22655,
                close: 1.22857,
            },
        };

        assert!(!is_in_corridor(&current_candle, &leading_candle, 20.0));
    }

    #[test]
    #[allow(non_snake_case)]
    fn crop_corridor_to_closest_leader__third_candle_is_appropriate_leader__new_existing_corridor()
    {
        let current_corridor = [
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.0,
                    high: 0.0,
                    low: 0.0,
                    close: 0.0,
                },
            },
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.1,
                    high: 0.1,
                    low: 0.1,
                    close: 0.1,
                },
            },
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.2,
                    high: 0.2,
                    low: 0.2,
                    close: 0.2,
                },
            },
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.3,
                    high: 0.3,
                    low: 0.3,
                    close: 0.3,
                },
            },
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.4,
                    high: 0.4,
                    low: 0.4,
                    close: 0.4,
                },
            },
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.5,
                    high: 0.5,
                    low: 0.5,
                    close: 0.5,
                },
            },
            BasicCandle {
                properties: Default::default(),
                edge_prices: CandleEdgePrices {
                    open: 0.6,
                    high: 0.6,
                    low: 0.6,
                    close: 0.6,
                },
            },
        ];

        let new_candle = BasicCandle {
            properties: Default::default(),
            edge_prices: CandleEdgePrices {
                open: 0.7,
                high: 0.7,
                low: 0.7,
                close: 0.7,
            },
        };
        let max_distance_from_corridor_leading_candle_pins_pct = 20.0;

        let number_of_calls_to_candle_can_be_corridor_leader = RefCell::new(0);
        let candle_can_be_corridor_leader = |_candle_properties: &CandleBaseProperties| {
            *number_of_calls_to_candle_can_be_corridor_leader.borrow_mut() += 1;
            *number_of_calls_to_candle_can_be_corridor_leader.borrow() > 1
        };

        let number_of_calls_to_is_in_corridor = RefCell::new(0);
        let is_in_corridor =
            |_candle: &BasicCandle,
             _leading_candle: &BasicCandle,
             _max_distance_from_corridor_leading_candle_pins_pct: PointSettingValue| {
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
