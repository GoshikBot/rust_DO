use crate::entities::CandleBaseProperties;
use crate::helpers::price_to_points;

/// Candle can be the corridor leader if its size is less or equal to the current volatility.
pub fn candle_can_be_corridor_leader(candle_properties: CandleBaseProperties) -> bool {
    if price_to_points(candle_properties.size) <= candle_properties.volatility {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::CandleType;
    use chrono::Utc;

    #[test]
    fn candle_can_be_corridor_leader() {
        let candle_properties = CandleBaseProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: 0.00150,
            volatility: 160.0,
        };

        assert!(super::candle_can_be_corridor_leader(candle_properties));
    }

    #[test]
    fn candle_cannot_be_corridor_leader() {
        let candle_properties = CandleBaseProperties {
            time: Utc::now().naive_utc(),
            r#type: CandleType::Green,
            size: 0.00180,
            volatility: 160.0,
        };

        assert!(!super::candle_can_be_corridor_leader(candle_properties));
    }
}
