use crate::step::utils::entities::working_levels::CorridorType;
use crate::step::utils::stores::working_level_store::StepWorkingLevelStore;
use anyhow::Result;
use base::entities::order::{OrderPrice, OrderType};
use base::entities::tick::TickPrice;
use base::params::ParamValue;

pub type MinAmountOfCandles = ParamValue;

pub trait LevelConditions {
    /// Checks whether the level exceeds the amount of candles in the corridor
    /// before the activation crossing of the level.
    fn level_exceeds_amount_of_candles_in_corridor(
        &self,
        level_id: &str,
        working_level_store: &impl StepWorkingLevelStore,
        corridor_type: CorridorType,
        min_amount_of_candles: MinAmountOfCandles,
    ) -> Result<bool>;

    fn price_is_beyond_stop_loss(
        &self,
        current_tick_price: TickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool;
}

#[derive(Default)]
pub struct LevelConditionsImpl;

impl LevelConditionsImpl {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LevelConditions for LevelConditionsImpl {
    fn level_exceeds_amount_of_candles_in_corridor(
        &self,
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
        &self,
        current_tick_price: TickPrice,
        stop_loss_price: OrderPrice,
        working_level_type: OrderType,
    ) -> bool {
        (working_level_type == OrderType::Buy && current_tick_price <= stop_loss_price)
            || working_level_type == OrderType::Sell && current_tick_price >= stop_loss_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::utils::entities::working_levels::{WLId, WLMaxCrossingValue};
    use base::entities::candle::{BasicCandleProperties, CandleId};
    use base::entities::order::OrderId;
    use base::entities::Item;
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

        fn move_working_level_to_removed(&mut self, _id: &str) -> Result<()> {
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

        fn get_removed_working_levels(
            &self,
        ) -> Result<Vec<Item<WLId, Self::WorkingLevelProperties>>> {
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

        fn move_take_profits_of_level(&mut self, _working_level_id: &str) -> Result<()> {
            unimplemented!()
        }

        fn are_take_profits_of_level_moved(&self, _working_level_id: &str) -> Result<bool> {
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

        let level_conditions = LevelConditionsImpl::default();

        let result = level_conditions
            .level_exceeds_amount_of_candles_in_corridor(
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

        let level_conditions = LevelConditionsImpl::default();

        let result = level_conditions
            .level_exceeds_amount_of_candles_in_corridor(
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

        let level_conditions = LevelConditionsImpl::default();

        let result = level_conditions
            .level_exceeds_amount_of_candles_in_corridor(
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

        let level_conditions = LevelConditionsImpl::default();

        let result = level_conditions
            .level_exceeds_amount_of_candles_in_corridor(
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
        let level_conditions = LevelConditionsImpl::default();

        assert!(level_conditions.price_is_beyond_stop_loss(
            dec!(1.38500),
            dec!(1.39000),
            OrderType::Buy
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__buy_level_current_tick_price_is_greater_than_stop_loss_price__should_return_false(
    ) {
        let level_conditions = LevelConditionsImpl::default();

        assert!(!level_conditions.price_is_beyond_stop_loss(
            dec!(1.39500),
            dec!(1.39000),
            OrderType::Buy
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_greater_than_stop_loss_price__should_return_true(
    ) {
        let level_conditions = LevelConditionsImpl::default();

        assert!(level_conditions.price_is_beyond_stop_loss(
            dec!(1.39500),
            dec!(1.39000),
            OrderType::Sell
        ));
    }

    #[test]
    #[allow(non_snake_case)]
    fn price_is_beyond_stop_loss__sell_level_current_tick_price_is_less_than_stop_loss_price__should_return_false(
    ) {
        let level_conditions = LevelConditionsImpl::default();

        assert!(!level_conditions.price_is_beyond_stop_loss(
            dec!(1.38500),
            dec!(1.39000),
            OrderType::Sell
        ));
    }
}
