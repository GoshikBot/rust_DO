use crate::entities::candle::BasicCandle;
use crate::entities::{BasicTick, HistoricalData};
use anyhow::{bail, Context, Result};
use chrono::NaiveDateTime;
use std::cmp::Ordering;

struct Candle {
    index: usize,
    time: NaiveDateTime,
}

fn find_tick_with_time(ticks: &[Option<BasicTick>], time: NaiveDateTime) -> Option<usize> {
    ticks
        .iter()
        .enumerate()
        .find_map(|(i, tick)| match tick.as_ref() {
            Some(tick) => {
                if tick.time == time {
                    Some(i)
                } else {
                    None
                }
            }
            None => None,
        })
}

/// Searches for the candle with the time greater or equal to the first tick time.
fn find_candle_around_first_tick(
    first_tick_time: NaiveDateTime,
    candles: &[Option<BasicCandle>],
) -> Option<Candle> {
    candles
        .iter()
        .enumerate()
        .find_map(|(i, candle)| match candle.as_ref() {
            Some(candle) => {
                if candle.properties.time >= first_tick_time {
                    Some(Candle {
                        index: i,
                        time: candle.properties.time,
                    })
                } else {
                    None
                }
            }
            None => None,
        })
}

/// Searches for the next not none candle.
fn find_next_candle(current_index: usize, candles: &[Option<BasicCandle>]) -> Result<Candle> {
    candles
        .iter()
        .enumerate()
        .skip(current_index + 1)
        .find_map(|(i, candle)| {
            candle.as_ref().map(|candle| Candle {
                index: i,
                time: candle.properties.time,
            })
        })
        .context("no next not none candle was found")
}

/// The sync process that can be called after positioning the candle_around_tick after the first tick.
fn sync_candles_and_ticks_after_positioning(
    mut historical_data: HistoricalData,
    mut candle_around_tick_time: NaiveDateTime,
    mut candle_around_tick_index: usize,
) -> Result<HistoricalData> {
    loop {
        let corresponding_tick_id =
            find_tick_with_time(&historical_data.ticks, candle_around_tick_time);

        match corresponding_tick_id {
            None => {
                Candle {
                    index: candle_around_tick_index,
                    time: candle_around_tick_time,
                } = find_next_candle(candle_around_tick_index, &historical_data.candles)?;
            }
            Some(tick_id) => {
                return Ok(HistoricalData {
                    candles: historical_data
                        .candles
                        .drain(candle_around_tick_index..)
                        .collect(),
                    ticks: historical_data.ticks.drain(tick_id..).collect(),
                });
            }
        }
    }
}

fn sync_front(mut historical_data: HistoricalData) -> Result<HistoricalData> {
    let first_candle_time = historical_data
        .candles
        .first()
        .context("no candles")?
        .as_ref()
        .context("first candle is None")?
        .properties
        .time;

    let first_tick_time = historical_data
        .ticks
        .first()
        .context("no ticks")?
        .as_ref()
        .context("first tick is None")?
        .time;

    match first_tick_time.cmp(&first_candle_time) {
        Ordering::Greater => {
            // position the first candle after the first tick before synchronization
            let Candle { index: candle_around_tick_index, time: candle_around_tick_time } = find_candle_around_first_tick(
                first_tick_time,
                &historical_data.candles
            ).context(
                format!("no candle around a first tick was found: historical data: {:#?}, first_tick_time: {}",
                        historical_data, first_tick_time)
            )?;

            if candle_around_tick_time == first_tick_time {
                return Ok(HistoricalData {
                    candles: historical_data
                        .candles
                        .drain(candle_around_tick_index..)
                        .collect(),
                    ticks: historical_data.ticks,
                });
            } else {
                sync_candles_and_ticks_after_positioning(
                    historical_data,
                    candle_around_tick_time,
                    candle_around_tick_index,
                )
            }
        }
        Ordering::Less => {
            // the first candle is already positioned after the first tick before synchronization
            sync_candles_and_ticks_after_positioning(historical_data, first_candle_time, 0)
        }
        Ordering::Equal => Ok(historical_data),
    }
}

fn find_tick_right_after_last_candle(
    ticks: &[Option<BasicTick>],
    last_candle_time: NaiveDateTime,
) -> Option<usize> {
    ticks.iter().enumerate().find_map(|(i, tick)| match tick {
        Some(tick) => {
            if tick.time > last_candle_time {
                Some(i)
            } else {
                None
            }
        }
        None => None,
    })
}

fn get_candles_before_last_tick(
    candles: Vec<Option<BasicCandle>>,
    last_tick_time: NaiveDateTime,
) -> Vec<Option<BasicCandle>> {
    candles
        .into_iter()
        .rev()
        .skip_while(|candle| match candle {
            Some(candle) => candle.properties.time >= last_tick_time,
            None => true,
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn sync_back(mut historical_data: HistoricalData) -> Result<HistoricalData> {
    let last_candle_time = historical_data
        .candles
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .properties
        .time;
    let last_tick_time = historical_data.ticks.last().unwrap().as_ref().unwrap().time;

    return match last_tick_time.cmp(&last_candle_time) {
        Ordering::Less | Ordering::Equal => Ok(HistoricalData {
            candles: get_candles_before_last_tick(historical_data.candles, last_tick_time),
            ticks: historical_data.ticks,
        }),
        Ordering::Greater => {
            let tick_right_after_last_candle =
                find_tick_right_after_last_candle(&historical_data.ticks, last_candle_time)
                    .unwrap();

            Ok(HistoricalData {
                candles: historical_data.candles,
                ticks: historical_data
                    .ticks
                    .drain(..=tick_right_after_last_candle)
                    .collect(),
            })
        }
    };
}

fn trim_historical_data(historical_data: HistoricalData) -> HistoricalData {
    HistoricalData {
        candles: historical_data
            .candles
            .into_iter()
            .rev()
            .skip_while(|candle| candle.is_none())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .skip_while(|candle| candle.is_none())
            .collect(),
        ticks: historical_data
            .ticks
            .into_iter()
            .rev()
            .skip_while(|tick| tick.is_none())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .skip_while(|tick| tick.is_none())
            .collect(),
    }
}

/// Reduces the first candle and the first tick to the same time.
pub fn sync_candles_and_ticks(mut historical_data: HistoricalData) -> Result<HistoricalData> {
    if historical_data.candles.is_empty() || historical_data.ticks.is_empty() {
        bail!("empty collection of items for synchronization");
    }

    let trimmed_historical_data = trim_historical_data(historical_data);

    let front_synchronized_data = sync_front(trimmed_historical_data)?;
    sync_back(front_synchronized_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::CandleBaseProperties;

    #[test]
    fn sync_candles_and_ticks_first_candle_before_first_tick_last_tick_after_last_candle_successfully(
    ) {
        let historical_data = HistoricalData {
            candles: vec![
                None,
                None,
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 10:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                None,
                None,
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                None,
                None,
            ],
            ticks: vec![
                None,
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 11:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 12:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 16:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                None,
            ],
        };

        let synchronized_historical_data =
            sync_candles_and_ticks(historical_data).unwrap_or_else(|e| panic!("{:?}", e));

        let expected_synchronized_historical_data = HistoricalData {
            candles: vec![
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
            ],
            ticks: vec![
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 16:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
            ],
        };

        assert_eq!(
            synchronized_historical_data,
            expected_synchronized_historical_data,
        );
    }

    #[test]
    fn sync_candles_and_ticks_first_tick_before_first_candle_last_candle_after_last_tick_successfully(
    ) {
        let historical_data = HistoricalData {
            candles: vec![
                None,
                None,
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 10:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                None,
                None,
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 16:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    },
                    edge_prices: Default::default(),
                }),
                None,
                None,
            ],
            ticks: vec![
                None,
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 08:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 09:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 10:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 11:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 12:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                None,
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                None,
                None,
            ],
        };

        let synchronized_historical_data =
            sync_candles_and_ticks(historical_data).unwrap_or_else(|e| panic!("{:?}", e));

        let expected_synchronized_historical_data = HistoricalData {
            candles: vec![Some(BasicCandle {
                properties: CandleBaseProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                },
                edge_prices: Default::default(),
            })],
            ticks: vec![
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 14:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
                Some(BasicTick {
                    time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ask: 0.0,
                    bid: 0.0,
                }),
            ],
        };

        assert_eq!(
            synchronized_historical_data,
            expected_synchronized_historical_data,
        );
    }
}
