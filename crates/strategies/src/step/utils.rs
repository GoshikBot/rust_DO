use self::stores::tick_store::StepTickStore;
use anyhow::Result;

pub mod backtesting_charts;
pub mod entities;
pub mod level_utils;
pub mod orders;
pub mod stores;
pub mod trading_limiter;

pub fn update_ticks<T>(
    new_tick: T,
    store: &mut impl StepTickStore<TickProperties = T>,
) -> Result<()> {
    let new_tick_id = store.create_tick(new_tick)?;

    if let Some(current_tick) = store.get_current_tick()? {
        store.update_previous_tick(current_tick.id)?;
    }

    store.update_current_tick(new_tick_id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use base::entities::BasicTickProperties;
    use base::stores::tick_store::BasicTickStore;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    use crate::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn update_ticks__current_tick_exists_in_store__should_update_previous_and_current_ticks() {
        let mut store = InMemoryStepBacktestingStore::default();

        let tick_id = store.create_tick(BasicTickProperties::default()).unwrap();
        store.update_current_tick(tick_id.clone()).unwrap();

        let new_tick = BasicTickProperties {
            time: Utc::now().naive_utc(),
            ask: dec!(10.5),
            bid: dec!(11.5),
        };

        update_ticks(new_tick.clone(), &mut store).unwrap();

        assert_eq!(store.get_previous_tick().unwrap().unwrap().id, tick_id);
        assert_eq!(store.get_current_tick().unwrap().unwrap().props, new_tick);
    }

    #[test]
    #[allow(non_snake_case)]
    fn update_ticks__no_current_tick_in_store__should_update_only_current_tick() {
        let mut store = InMemoryStepBacktestingStore::default();

        let new_tick = BasicTickProperties::default();

        update_ticks(new_tick.clone(), &mut store).unwrap();

        assert_eq!(store.get_current_tick().unwrap().unwrap().props, new_tick);
        assert!(store.get_previous_tick().unwrap().is_none());
    }
}
