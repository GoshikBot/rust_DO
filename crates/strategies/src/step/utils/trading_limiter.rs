use base::entities::tick::{HistoricalTickPrice, TickPrice};
use base::entities::BasicTickProperties;
use chrono::{NaiveDateTime, Timelike};
use std::ops::Range;

const HOUR_TO_FORBID_TRADING: u8 = 23;
const _HOURS_TO_START_CHECKING_TO_ALLOW_TRADING_REALTIME: Range<u8> = 0..23;

const HOURS_TO_FORBID_TRADING_BACKTESTING: [u8; 3] = [23, 0, 1];

pub trait TradingLimiter {
    type TickPrice;

    fn forbid_trading(&self, current_tick: &BasicTickProperties<Self::TickPrice>) -> bool;
    fn allow_trading(&self, current_tick: &BasicTickProperties<Self::TickPrice>) -> bool;
}

fn forbid_trading(current_tick_time: NaiveDateTime) -> bool {
    if current_tick_time.time().hour() as u8 == HOUR_TO_FORBID_TRADING {
        return true;
    }

    false
}

#[derive(Default)]
pub struct TradingLimiterBacktesting;

impl TradingLimiterBacktesting {
    pub fn new() -> Self {
        Default::default()
    }
}

impl TradingLimiter for TradingLimiterBacktesting {
    type TickPrice = HistoricalTickPrice;

    fn forbid_trading(&self, current_tick: &BasicTickProperties<Self::TickPrice>) -> bool {
        forbid_trading(current_tick.time)
    }

    /// Backtesting doesn't check for the appropriate spread to allow trading,
    /// so we just mark hours when we don't want to have trading. At any time
    /// beyond these hours the trading for backtesting will become allowed.
    fn allow_trading(&self, current_tick: &BasicTickProperties<Self::TickPrice>) -> bool {
        if HOURS_TO_FORBID_TRADING_BACKTESTING.contains(&(current_tick.time.time().hour() as u8)) {
            return false;
        }

        true
    }
}

#[derive(Default)]
pub struct TradingLimiterRealtime;

impl TradingLimiterRealtime {
    pub fn new() -> Self {
        Default::default()
    }
}

impl TradingLimiter for TradingLimiterRealtime {
    type TickPrice = TickPrice;

    fn forbid_trading(&self, current_tick: &BasicTickProperties<Self::TickPrice>) -> bool {
        forbid_trading(current_tick.time)
    }

    /// For realtime we mark the hour to forbid trading and denote hours
    /// to start checking for the satisfactory spread to allow trading again.
    fn allow_trading(&self, _current_tick: &BasicTickProperties<Self::TickPrice>) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::collections::HashSet;

    #[test]
    fn forbid_trading_hour_to_forbid_trading_return_true() {
        let current_tick_time = NaiveDate::from_ymd(2022, 5, 28).and_hms(23, 0, 0);

        assert!(forbid_trading(current_tick_time));
    }

    #[test]
    fn forbid_trading_hours_when_not_to_forbid_trading_return_false() {
        let tick_times_when_not_to_forbid_trading = (0..23)
            .map(|n| NaiveDate::from_ymd(2022, 5, 28).and_hms(n, 0, 0))
            .collect::<Vec<_>>();

        for tick_time in tick_times_when_not_to_forbid_trading {
            assert!(!forbid_trading(tick_time));
        }
    }

    #[test]
    fn allow_trading_backtesting_at_hours_to_allow_trading_return_true() {
        let ticks_when_to_allow_trading = (0..=23)
            .collect::<HashSet<u8>>()
            .difference(
                &HOURS_TO_FORBID_TRADING_BACKTESTING
                    .into_iter()
                    .collect::<HashSet<_>>(),
            )
            .map(|&n| BasicTickProperties {
                time: NaiveDate::from_ymd(2022, 5, 25).and_hms(n as u32, 0, 0),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let trading_limiter = TradingLimiterBacktesting::new();

        for tick in ticks_when_to_allow_trading {
            assert!(trading_limiter.allow_trading(&tick));
        }
    }

    #[test]
    fn allow_trading_backtesting_at_hours_to_forbid_trading_return_false() {
        let ticks_when_to_forbid_trading =
            HOURS_TO_FORBID_TRADING_BACKTESTING.map(|n| BasicTickProperties {
                time: NaiveDate::from_ymd(2022, 5, 25).and_hms(n as u32, 0, 0),
                ..Default::default()
            });

        let trading_limiter = TradingLimiterBacktesting::new();

        for tick in ticks_when_to_forbid_trading {
            assert!(!trading_limiter.allow_trading(&tick));
        }
    }
}
