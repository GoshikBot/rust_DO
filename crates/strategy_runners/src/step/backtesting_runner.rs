use anyhow::Context;
use anyhow::Result;
use backtesting::HistoricalData;
use base::entities::candle::BasicCandle;
use base::entities::{BasicTick, StrategyTimeframes, Timeframe};
use base::params::StrategyParams;
use strategies::step::utils::entities::{StrategyPerformance, StrategySignals};
use strategies::step::utils::stores::{StepBacktestingBalances, StepBacktestingStores};
use strategies::step::utils::trading_limiter;
use strategies::step::utils::trading_limiter::TradingLimiter;

#[derive(Debug)]
struct Tick<'a> {
    index: usize,
    value: Option<&'a BasicTick>,
}

#[derive(Debug)]
struct Candle<'a> {
    index: usize,
    value: Option<&'a BasicCandle>,
}

fn update_number_of_iterations_to_next_candle(
    number_of_iterations_to_next_candle: &mut u32,
    number_of_iterations_between_candles: u32,
) {
    if *number_of_iterations_to_next_candle == 0 {
        *number_of_iterations_to_next_candle = number_of_iterations_between_candles - 1;
    } else {
        *number_of_iterations_to_next_candle -= 1;
    }
}

fn strategy_performance(balances: &StepBacktestingBalances) -> StrategyPerformance {
    (balances.real - balances.initial) / balances.initial * 100.0
}

pub struct StepStrategyRunningConfig<'a, P>
where
    P: StrategyParams,
{
    pub timeframes: StrategyTimeframes,
    pub stores: &'a mut StepBacktestingStores,
    pub params: &'a P,
}

pub fn loop_through_historical_data<P, L, R>(
    historical_data: &HistoricalData,
    strategy_config: StepStrategyRunningConfig<P>,
    trading_limiter: &L,
    run_iteration: R,
) -> Result<StrategyPerformance>
where
    P: StrategyParams,
    L: TradingLimiter,
    R: Fn(
        &BasicTick,
        Option<&BasicCandle>,
        StrategySignals,
        &mut StepBacktestingStores,
        &P,
    ) -> Result<()>,
{
    let mut current_tick = Tick {
        index: 0,
        value: historical_data
            .ticks
            .get(0)
            .context("no first tick")?
            .as_ref(),
    };

    let mut current_candle = Candle {
        index: 0,
        value: historical_data
            .candles
            .get(0)
            .context("no first candle")?
            .as_ref(),
    };

    let mut first_candle = true;

    let mut new_candle_appeared = false;

    let mut no_trading_mode = false;
    let mut cancel_all_orders = false;

    let number_of_iterations_between_candles =
        strategy_config.timeframes.candle as u32 / strategy_config.timeframes.tick as u32;
    let mut number_of_iterations_to_next_candle = number_of_iterations_between_candles - 1;

    loop {
        if let Some(current_tick) = current_tick.value {
            if no_trading_mode {
                if trading_limiter.allow_trading(current_tick) {
                    no_trading_mode = false;
                }
            } else if trading_limiter.forbid_trading(current_tick) {
                no_trading_mode = true;
                cancel_all_orders = true;
            }

            // run iteration only if a tick exists
            run_iteration(
                current_tick,
                if new_candle_appeared {
                    current_candle.value
                } else {
                    None
                },
                StrategySignals {
                    no_trading_mode,
                    cancel_all_orders,
                },
                strategy_config.stores,
                strategy_config.params,
            )?;

            if cancel_all_orders {
                cancel_all_orders = false;
            }
        }

        if new_candle_appeared {
            new_candle_appeared = false;
        }

        update_number_of_iterations_to_next_candle(
            &mut number_of_iterations_to_next_candle,
            number_of_iterations_between_candles,
        );

        // the moment to update the current candle
        if number_of_iterations_to_next_candle == 0 {
            if !first_candle {
                let new_candle_value = historical_data.candles.get(current_candle.index + 1);
                match new_candle_value {
                    Some(new_candle) => {
                        current_candle = Candle {
                            index: current_candle.index + 1,
                            value: new_candle.as_ref(),
                        };
                    }
                    None => break,
                }
            } else {
                first_candle = false;
            }

            new_candle_appeared = true;
        }

        // the moment to update the current tick
        let new_tick_value = historical_data.ticks.get(current_tick.index + 1);
        match new_tick_value {
            None => break,
            Some(next_tick_value) => {
                current_tick = Tick {
                    index: current_tick.index + 1,
                    value: next_tick_value.as_ref(),
                };
            }
        }
    }

    Ok(strategy_performance(
        &strategy_config.stores.config.balances,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::entities::candle::CandleVolatility;
    use base::entities::CandleBaseProperties;
    use base::params::PointSettingValue;
    use chrono::{NaiveDateTime, Timelike};
    use float_cmp::approx_eq;

    const HOUR_TO_FORBID_TRADING: u8 = 23;
    const HOURS_TO_FORBID_TRADING: [u8; 3] = [23, 0, 1];

    #[derive(Default)]
    struct TestTradingLimiter;

    impl TestTradingLimiter {
        fn new() -> Self {
            Default::default()
        }
    }

    impl TradingLimiter for TestTradingLimiter {
        fn forbid_trading(&self, current_tick: &BasicTick) -> bool {
            if current_tick.time.time().hour() as u8 == HOUR_TO_FORBID_TRADING {
                return true;
            }

            false
        }

        fn allow_trading(&self, current_tick: &BasicTick) -> bool {
            if HOURS_TO_FORBID_TRADING.contains(&(current_tick.time.time().hour() as u8)) {
                return false;
            }

            true
        }
    }

    fn run_iteration(
        tick: &BasicTick,
        candle: Option<&BasicCandle>,
        signals: StrategySignals,
        stores: &mut StepBacktestingStores,
        _params: &impl StrategyParams,
    ) -> Result<()> {
        if signals.cancel_all_orders {
            stores.config.balances.real -= 50.0;
        }

        if !signals.no_trading_mode {
            stores.config.balances.real += 10.0;

            if candle.is_some() {
                stores.config.balances.real += 20.0;
            }
        }

        Ok(())
    }

    #[derive(Default)]
    struct TestStrategyParams;

    impl TestStrategyParams {
        fn new() -> Self {
            Default::default()
        }
    }

    impl StrategyParams for TestStrategyParams {
        type PointParam = String;
        type RatioParam = String;

        fn get_point_param_value(&self, _name: Self::PointParam) -> PointSettingValue {
            todo!()
        }

        fn get_ratio_param_value(
            &self,
            _name: Self::RatioParam,
            _volatility: CandleVolatility,
        ) -> PointSettingValue {
            todo!()
        }
    }

    #[test]
    fn loop_through_historical_data_proper_params_get_correct_performance() {
        let historical_data = HistoricalData {
            candles: vec![
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 18:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                None,
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 20:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 21:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 22:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 23:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 00:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 01:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                None,
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 03:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 04:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("18-05-2022 05:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
            ],
            ticks: vec![
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 18:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 19:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 19:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 20:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 20:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 21:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 22:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 22:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 23:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 23:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 00:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 00:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 01:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 02:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 02:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 03:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 03:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 04:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 04:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 05:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 05:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("18-05-2022 06:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
            ],
        };

        let mut step_stores: StepBacktestingStores = Default::default();
        let step_params = TestStrategyParams::new();

        let trading_limiter = TestTradingLimiter::new();

        let strategy_config = StepStrategyRunningConfig {
            timeframes: StrategyTimeframes {
                candle: Timeframe::Hour,
                tick: Timeframe::ThirtyMin,
            },
            stores: &mut step_stores,
            params: &step_params,
        };

        let strategy_performance = loop_through_historical_data(
            &historical_data,
            strategy_config,
            &trading_limiter,
            run_iteration,
        )
        .unwrap();

        assert!(approx_eq!(f32, strategy_performance, 2.6));
        assert!(approx_eq!(f32, step_stores.config.balances.real, 10_260.0));
    }
}
