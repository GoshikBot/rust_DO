use crate::step::utils::entities::working_levels::{
    BasicWLProperties, CorridorType, LevelTime, WLMaxCrossingValue, WLPrice,
};
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::candle::CandleVolatility;
use base::entities::order::{BasicOrderProperties, OrderPrice, OrderStatus, OrderType};
use base::entities::tick::{TickPrice, TickTime};
use base::entities::{DEFAULT_HOLIDAYS, TARGET_LOGGER_ENV};
use base::helpers::{price_to_points, Holiday, NumberOfDaysToExclude};
use base::params::ParamValue;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;

pub type MinAmountOfCandles = ParamValue;

pub trait LevelConditions {
    /// Checks whether the level exceeds the amount of candles in the corridor
    /// before the activation crossing of the level.
    fn level_exceeds_amount_of_candles_in_corridor(
        level_id: &str,
        working_level_store: &impl StepWorkingLevelStore,
        corridor_type: CorridorType,
        min_amount_of_candles: MinAmountOfCandles,
    ) -> Result<bool>;

    fn price_is_beyond_stop_loss(
        current_tick_price: TickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool;

    fn level_expired_by_distance(
        level_price: WLPrice,
        current_tick_price: TickPrice,
        distance_from_level_for_its_deletion: ParamValue,
    ) -> bool;

    fn level_expired_by_time(
        level_time: LevelTime,
        current_tick_time: TickTime,
        level_expiration: ParamValue,
        exclude_weekend_and_holidays: &impl Fn(
            NaiveDateTime,
            NaiveDateTime,
            &[Holiday],
        ) -> NumberOfDaysToExclude,
    ) -> bool;

    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        level: &impl AsRef<BasicWLProperties>,
        max_crossing_value: Option<WLMaxCrossingValue>,
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
        current_tick_price: TickPrice,
    ) -> bool;

    fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool;
}

#[derive(Default)]
pub struct LevelConditionsImpl;

impl LevelConditions for LevelConditionsImpl {
    fn level_exceeds_amount_of_candles_in_corridor(
        level_id: &str,
        working_level_store: &impl StepWorkingLevelStore,
        corridor_type: CorridorType,
        min_amount_of_candles: MinAmountOfCandles,
    ) -> Result<bool> {
        let corridor =
            working_level_store.get_candles_of_working_level_corridor(level_id, corridor_type)?;

        Ok(ParamValue::from(corridor.len()) >= min_amount_of_candles)
    }

    fn price_is_beyond_stop_loss(
        current_tick_price: TickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool {
        (working_level_type == OrderType::Buy && current_tick_price <= stop_loss_price)
            || working_level_type == OrderType::Sell && current_tick_price >= stop_loss_price
    }

    fn level_expired_by_distance(
        level_price: WLPrice,
        current_tick_price: TickPrice,
        distance_from_level_for_its_deletion: ParamValue,
    ) -> bool {
        log::debug!(
            target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
            "level_expired_by_distance: level price is {}, current tick price is {}, \
            distance from level for its deletion is {}",
            level_price, current_tick_price, distance_from_level_for_its_deletion
        );

        price_to_points((level_price - current_tick_price).abs())
            >= distance_from_level_for_its_deletion
    }

    fn level_expired_by_time(
        level_time: LevelTime,
        current_tick_time: TickTime,
        level_expiration: ParamValue,
        exclude_weekend_and_holidays: &impl Fn(
            NaiveDateTime,
            NaiveDateTime,
            &[Holiday],
        ) -> NumberOfDaysToExclude,
    ) -> bool {
        let diff = (current_tick_time - level_time).num_days()
            - exclude_weekend_and_holidays(level_time, current_tick_time, &DEFAULT_HOLIDAYS) as i64;

        log::debug!(
            target: &dotenv::var(TARGET_LOGGER_ENV).unwrap(),
            "level_expired_by_time: current tick time is {}, level time is {},\
            level expiration is {}, diff is {}",
            current_tick_time, level_time, level_expiration, diff
        );

        Decimal::from(diff) >= level_expiration
    }

    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level(
        level: &impl AsRef<BasicWLProperties>,
        max_crossing_value: Option<WLMaxCrossingValue>,
        min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion: ParamValue,
        current_tick_price: TickPrice,
    ) -> bool {
        let level = level.as_ref();

        if (level.r#type == OrderType::Buy && current_tick_price >= level.price)
            || (level.r#type == OrderType::Sell && current_tick_price <= level.price)
        {
            if let Some(max_crossing_value) = max_crossing_value {
                if max_crossing_value >= min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion {
                    return true;
                }
            }
        }

        false
    }

    fn level_has_no_active_orders(level_orders: &[impl AsRef<BasicOrderProperties>]) -> bool {
        for order in level_orders {
            if order.as_ref().status != OrderStatus::Pending {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::utils::entities::working_levels::{WLId, WLMaxCrossingValue, WLStatus};
    use base::entities::candle::{BasicCandleProperties, CandleId};
    use base::entities::order::OrderId;
    use base::entities::Item;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    struct TestWorkingLevelStore {
        small_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
        big_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
    }

    impl TestWorkingLevelStore {
        fn default(
            small_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
            big_corridor: Vec<Item<CandleId, <Self as StepWorkingLevelStore>::CandleProperties>>,
        ) -> Self {
            Self {
                small_corridor,
                big_corridor,
            }
        }
    }

    impl StepWorkingLevelStore for TestWorkingLevelStore {
        type WorkingLevelProperties = ();
        type CandleProperties = BasicCandleProperties;
        type OrderProperties = ();

        fn create_working_level(
            &mut self,
            _properties: Self::WorkingLevelProperties,
        ) -> Result<Item<WLId, Self::WorkingLevelProperties>> {
            unimplemented!()
        }

        fn get_working_level_by_id(
            &self,
            _id: &str,
        ) -> Result<Option<Item<WLId, Self::WorkingLevelProperties>>> {
            unimplemented!()
        }

        fn move_working_level_to_active(&mut self, _id: &str) -> Result<()> {
            unimplemented!()
        }

        fn remove_working_level(&mut self, _id: &str) -> Result<()> {
            unimplemented!()
        }

        fn get_created_working_levels(
            &self,
        ) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
            unimplemented!()
        }

        fn get_active_working_levels(
            &self,
        ) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
            unimplemented!()
        }

        fn get_working_level_status(&self, id: &str) -> Result<Option<WLStatus>> {
            unimplemented!()
        }

        fn clear_working_level_corridor(
            &mut self,
            working_level_id: &str,
            corridor_type: CorridorType,
        ) -> Result<()> {
            unimplemented!()
        }

        fn add_candle_to_working_level_corridor(
            &mut self,
            _working_level_id: &str,
            _candle_id: CandleId,
            _corridor_type: CorridorType,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_candles_of_working_level_corridor(
            &self,
            _working_level_id: &str,
            corridor_type: CorridorType,
        ) -> Result<Vec<Item<CandleId, Self::CandleProperties>>> {
            Ok(match corridor_type {
                CorridorType::Small => self.small_corridor.clone(),
                CorridorType::Big => self.big_corridor.clone(),
            })
        }

        fn update_max_crossing_value_of_working_level(
            &mut self,
            _working_level_id: &str,
            _default_value: WLMaxCrossingValue,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_max_crossing_value_of_working_level(
            &self,
            _working_level_id: &str,
        ) -> Result<Option<WLMaxCrossingValue>> {
            unimplemented!()
        }

        fn move_take_profits_of_level(
            &mut self,
            working_level_id: &str,
            distance_to_move_take_profits: ParamValue,
        ) -> Result<()> {
            unimplemented!()
        }

        fn take_profits_of_level_are_moved(&self, _working_level_id: &str) -> Result<bool> {
            unimplemented!()
        }

        fn add_order_to_working_level_chain_of_orders(
            &mut self,
            _working_level_id: &str,
            _order_id: OrderId,
        ) -> Result<()> {
            unimplemented!()
        }

        fn get_working_level_chain_of_orders(
            &self,
            _working_level_id: &str,
        ) -> Result<Vec<Item<OrderId, Self::OrderProperties>>> {
            unimplemented!()
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_small_corridor_is_greater_than_min_amount_of_candles__should_return_true(
    ) {
        let small_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("3"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("4"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("5"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(small_corridor, vec![]);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Small,
            dec!(3),
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_small_corridor_is_less_than_min_amount_of_candles__should_return_false(
    ) {
        let small_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(small_corridor, vec![]);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Small,
            dec!(3),
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_big_corridor_is_greater_than_min_amount_of_candles__should_return_true(
    ) {
        let big_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("3"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("4"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("5"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(vec![], big_corridor);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Big,
            dec!(3),
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_exceeds_amount_of_candles_in_corridor__len_of_big_corridor_is_less_than_min_amount_of_candles__should_return_false(
    ) {
        let big_corridor = vec![
            Item {
                id: String::from("1"),
                props: BasicCandleProperties::default(),
            },
            Item {
                id: String::from("2"),
                props: BasicCandleProperties::default(),
            },
        ];

        let working_level_store = TestWorkingLevelStore::default(vec![], big_corridor);
        let level_id = "1";

        let result = LevelConditionsImpl::level_exceeds_amount_of_candles_in_corridor(
            level_id,
            &working_level_store,
            CorridorType::Big,
            dec!(3),
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_less_than_stop_loss_price__should_return_true(
    ) {
        assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.38500),
            dec!(1.39000),
            OrderType::Buy
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_greater_than_stop_loss_price__should_return_false(
    ) {
        assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.39500),
            dec!(1.39000),
            OrderType::Buy
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_greater_than_stop_loss_price__should_return_true(
    ) {
        assert!(LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.39500),
            dec!(1.39000),
            OrderType::Sell
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_less_than_stop_loss_price__should_return_false(
    ) {
        assert!(!LevelConditionsImpl::price_is_beyond_stop_loss(
            dec!(1.38500),
            dec!(1.39000),
            OrderType::Sell
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_expired_by_distance__current_tick_price_is_in_acceptable_range_from_level_price__should_return_false(
    ) {
        assert!(!LevelConditionsImpl::level_expired_by_distance(
            dec!(1.38000),
            dec!(1.39000),
            dec!(2_000)
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_expired_by_distance__current_tick_price_is_beyond_acceptable_range_from_level_price__should_return_true(
    ) {
        assert!(LevelConditionsImpl::level_expired_by_distance(
            dec!(1.38000),
            dec!(1.40001),
            dec!(2_000)
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_expired_by_time__current_diff_is_greater_than_level_expiration__should_return_true() {
        let level_time = NaiveDate::from_ymd(2022, 8, 11).and_hms(0, 0, 0);
        let current_tick_time = NaiveDate::from_ymd(2022, 8, 19).and_hms(0, 0, 0);
        let level_expiration = dec!(5);

        let exclude_weekend_and_holidays =
            |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 2;

        assert!(LevelConditionsImpl::level_expired_by_time(
            level_time,
            current_tick_time,
            level_expiration,
            &exclude_weekend_and_holidays
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_expired_by_time__current_diff_is_less_than_level_expiration__should_return_false() {
        let level_time = NaiveDate::from_ymd(2022, 8, 11).and_hms(0, 0, 0);
        let current_tick_time = NaiveDate::from_ymd(2022, 8, 19).and_hms(0, 0, 0);
        let level_expiration = dec!(7);

        let exclude_weekend_and_holidays =
            |_start_time: NaiveDateTime, _end_time: NaiveDateTime, _holidays: &[Holiday]| 2;

        assert!(!LevelConditionsImpl::level_expired_by_time(
            level_time,
            current_tick_time,
            level_expiration,
            &exclude_weekend_and_holidays
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_has_no_opened_orders__all_orders_are_pending__should_return_true() {
        let orders = vec![
            BasicOrderProperties::default(),
            BasicOrderProperties::default(),
            BasicOrderProperties::default(),
            BasicOrderProperties::default(),
        ];

        assert!(LevelConditionsImpl::level_has_no_active_orders(&orders));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_has_no_opened_orders__some_orders_are_opened__should_return_false() {
        let orders = vec![
            BasicOrderProperties::default(),
            BasicOrderProperties::default(),
            BasicOrderProperties {
                status: OrderStatus::Opened,
                ..Default::default()
            },
            BasicOrderProperties::default(),
        ];

        assert!(!LevelConditionsImpl::level_has_no_active_orders(&orders));
    }

    #[test]
    #[allow(non_snake_case)]
    fn level_has_no_opened_orders__some_orders_are_closed__should_return_false() {
        let orders = vec![
            BasicOrderProperties::default(),
            BasicOrderProperties::default(),
            BasicOrderProperties {
                status: OrderStatus::Closed,
                ..Default::default()
            },
            BasicOrderProperties::default(),
        ];

        assert!(!LevelConditionsImpl::level_has_no_active_orders(&orders));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_buy_level_max_crossing_value_is_beyond_limit__should_return_true(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Buy,
            ..Default::default()
        };

        let max_crossing_value = dec!(200);
        let current_tick_price = dec!(1.38050);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(
            LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__have_not_returned_to_buy_level_max_crossing_value_is_beyond_limit__should_return_false(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Buy,
            ..Default::default()
        };

        let max_crossing_value = dec!(200);
        let current_tick_price = dec!(1.37999);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_buy_level_max_crossing_value_is_not_beyond_limit__should_return_false(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Buy,
            ..Default::default()
        };

        let max_crossing_value = dec!(99);
        let current_tick_price = dec!(1.38050);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_sell_level_max_crossing_value_is_beyond_limit__should_return_true(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Sell,
            ..Default::default()
        };

        let max_crossing_value = dec!(200);
        let current_tick_price = dec!(1.37999);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__returned_to_sell_level_max_crossing_value_is_not_beyond_limit__should_return_false(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Sell,
            ..Default::default()
        };

        let max_crossing_value = dec!(50);
        let current_tick_price = dec!(1.37999);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn active_level_exceeds_activation_crossing_distance_when_returned_to_level__have_not_returned_to_sell_level_max_crossing_value_is_beyond_limit__should_return_false(
    ) {
        let level = BasicWLProperties {
            price: dec!(1.38000),
            r#type: OrderType::Sell,
            ..Default::default()
        };

        let max_crossing_value = dec!(200);
        let current_tick_price = dec!(1.38001);
        let min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion =
            dec!(100);

        assert!(!LevelConditionsImpl::active_level_exceeds_activation_crossing_distance_when_returned_to_level(
            &level,
            Some(max_crossing_value),
            min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion,
            current_tick_price,
        ));
    }
}
