use crate::step::utils::entities::candle::StepCandleProperties;
use crate::step::utils::entities::Diff;
use std::cmp::Ordering;

pub trait AngleUtils {
    /// Calculates the difference between current and previous candle leading prices
    /// to further determine angles.
    fn get_diff_between_current_and_previous_candles<C>(
        current_candle_props: &C,
        previous_candle_props: &C,
    ) -> Diff
    where
        C: AsRef<StepCandleProperties>;
}

pub struct AngleUtilsImpl;

impl AngleUtils for AngleUtilsImpl {
    fn get_diff_between_current_and_previous_candles<C>(
        current_candle_props: &C,
        previous_candle_props: &C,
    ) -> Diff
    where
        C: AsRef<StepCandleProperties>,
    {
        let current_candle_props = current_candle_props.as_ref();
        let previous_candle_props = previous_candle_props.as_ref();

        match current_candle_props
            .leading_price
            .cmp(&previous_candle_props.leading_price)
        {
            Ordering::Greater => Diff::Greater,
            Ordering::Less => Diff::Less,
            Ordering::Equal => {
                if current_candle_props.leading_price == current_candle_props.base.prices.high {
                    Diff::Greater
                } else {
                    Diff::Less
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::entities::candle::BasicCandleProperties;
    use base::entities::CandlePrices;
    use rust_decimal_macros::dec;

    #[test]
    #[allow(non_snake_case)]
    fn get_diff_between_current_and_previous_candles__current_candle_is_greater_than_previous__should_return_greater(
    ) {
        let current_candle_props = StepCandleProperties {
            leading_price: dec!(1.38000),
            ..Default::default()
        };
        let previous_candle_props = StepCandleProperties {
            leading_price: dec!(1.37950),
            ..Default::default()
        };

        assert_eq!(
            AngleUtilsImpl::get_diff_between_current_and_previous_candles(
                &current_candle_props,
                &previous_candle_props
            ),
            Diff::Greater
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_diff_between_current_and_previous_candles__current_candle_is_less_than_previous__should_return_greater(
    ) {
        let current_candle_props = StepCandleProperties {
            leading_price: dec!(1.38000),
            ..Default::default()
        };
        let previous_candle_props = StepCandleProperties {
            leading_price: dec!(1.38100),
            ..Default::default()
        };

        assert_eq!(
            AngleUtilsImpl::get_diff_between_current_and_previous_candles(
                &current_candle_props,
                &previous_candle_props
            ),
            Diff::Less
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_diff_between_current_and_previous_candles__current_candle_is_equal_to_previous_and_leading_price_is_equal_to_high__should_return_greater(
    ) {
        let current_candle_props = StepCandleProperties {
            leading_price: dec!(1.38000),
            base: BasicCandleProperties {
                prices: CandlePrices {
                    high: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        };
        let previous_candle_props = StepCandleProperties {
            leading_price: dec!(1.38000),
            ..Default::default()
        };

        assert_eq!(
            AngleUtilsImpl::get_diff_between_current_and_previous_candles(
                &current_candle_props,
                &previous_candle_props
            ),
            Diff::Greater
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn get_diff_between_current_and_previous_candles__current_candle_is_equal_to_previous_and_leading_price_is_equal_to_low__should_return_less(
    ) {
        let current_candle_props = StepCandleProperties {
            leading_price: dec!(1.38000),
            base: BasicCandleProperties {
                prices: CandlePrices {
                    low: dec!(1.38000),
                    ..Default::default()
                },
                ..Default::default()
            },
        };
        let previous_candle_props = StepCandleProperties {
            leading_price: dec!(1.38000),
            ..Default::default()
        };

        assert_eq!(
            AngleUtilsImpl::get_diff_between_current_and_previous_candles(
                &current_candle_props,
                &previous_candle_props
            ),
            Diff::Less
        );
    }
}
