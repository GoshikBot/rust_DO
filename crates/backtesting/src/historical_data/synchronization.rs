use std::cmp::Ordering;

use anyhow::{bail, Context, Result};
use chrono::NaiveDateTime;

use base::entities::candle::BasicCandle;
use base::entities::BasicTick;

use crate::HistoricalData;

#[derive(Copy, Clone)]
struct Candle {
    index: usize,
    time: NaiveDateTime,
}

fn find_tick_with_time<'a>(
    ticks: impl Iterator<Item = &'a Option<BasicTick>>,
    time: NaiveDateTime,
) -> Option<usize> {
    ticks.enumerate().find_map(|(i, tick)| match tick.as_ref() {
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

/// Searches for the candle with the time greater or equal to the tick time.
fn find_candle_around_tick<'a>(
    first_tick_time: NaiveDateTime,
    candle_iterator: impl Iterator<Item = &'a Option<BasicCandle>>,
) -> Option<Candle> {
    candle_iterator
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
fn find_next_candle<'a>(
    current_index: usize,
    candles: impl Iterator<Item = &'a Option<BasicCandle>>,
) -> Result<Candle> {
    candles
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

struct TickCandle {
    tick_index: usize,
    candle_index: usize,
}

struct Intersection {
    front: TickCandle,
    back: TickCandle,
}

fn find_timeframe_equal_times<'a, 'b, C, T>(
    candle_iterator: C,
    tick_iterator: T,
    mut candle_around_tick: Candle,
) -> Result<TickCandle>
where
    C: Iterator<Item = &'a Option<BasicCandle>> + Clone,
    T: Iterator<Item = &'b Option<BasicTick>> + Clone,
{
    loop {
        let corresponding_tick_index =
            find_tick_with_time(tick_iterator.clone(), candle_around_tick.time);

        match corresponding_tick_index {
            None => {
                candle_around_tick =
                    find_next_candle(candle_around_tick.index, candle_iterator.clone())?;
            }
            Some(tick_index) => {
                return Ok(TickCandle {
                    tick_index,
                    candle_index: candle_around_tick.index,
                });
            }
        }
    }
}

enum Edge {
    Front,
    Back,
}

fn reverse_edge_intersection_indexes(
    ticks_len: usize,
    candles_len: usize,
    intersection: TickCandle,
) -> TickCandle {
    TickCandle {
        tick_index: ticks_len - intersection.tick_index - 1,
        candle_index: candles_len - intersection.candle_index - 1,
    }
}

fn find_edge_intersection(historical_data: &HistoricalData, edge: Edge) -> Result<TickCandle> {
    let first_candle_time = historical_data
        .candles
        .first()
        .unwrap()
        .as_ref()
        .unwrap()
        .properties
        .time;
    let first_tick_time = historical_data
        .ticks
        .first()
        .unwrap()
        .as_ref()
        .unwrap()
        .time;

    return match first_tick_time.cmp(&first_candle_time) {
        Ordering::Greater => {
            let candle_around_first_tick = match edge {
                Edge::Front => {
                    find_candle_around_tick(first_tick_time, historical_data.candles.iter())
                }
                Edge::Back => {
                    find_candle_around_tick(first_tick_time, historical_data.candles.iter().rev())
                }
            }
            .context("no candle around a first tick was found")?;

            return if candle_around_first_tick.time == first_tick_time {
                let intersection = TickCandle {
                    tick_index: 0,
                    candle_index: candle_around_first_tick.index,
                };

                match edge {
                    Edge::Front => Ok(intersection),
                    Edge::Back => Ok(reverse_edge_intersection_indexes(
                        historical_data.ticks.len(),
                        historical_data.candles.len(),
                        intersection,
                    )),
                }
            } else {
                match edge {
                    Edge::Front => find_timeframe_equal_times(
                        historical_data.candles.iter(),
                        historical_data.ticks.iter(),
                        candle_around_first_tick,
                    ),
                    Edge::Back => Ok(reverse_edge_intersection_indexes(
                        historical_data.ticks.len(),
                        historical_data.candles.len(),
                        find_timeframe_equal_times(
                            historical_data.candles.iter().rev(),
                            historical_data.ticks.iter().rev(),
                            candle_around_first_tick,
                        )?,
                    )),
                }
            };
        }
        Ordering::Less => {
            let candle_around_tick = Candle {
                index: 0,
                time: first_candle_time,
            };

            match edge {
                Edge::Front => find_timeframe_equal_times(
                    historical_data.candles.iter(),
                    historical_data.ticks.iter(),
                    candle_around_tick,
                ),
                Edge::Back => Ok(reverse_edge_intersection_indexes(
                    historical_data.ticks.len(),
                    historical_data.candles.len(),
                    find_timeframe_equal_times(
                        historical_data.candles.iter().rev(),
                        historical_data.ticks.iter().rev(),
                        candle_around_tick,
                    )?,
                )),
            }
        }
        Ordering::Equal => {
            let intersection = TickCandle {
                tick_index: 0,
                candle_index: 0,
            };

            match edge {
                Edge::Front => Ok(intersection),
                Edge::Back => Ok(reverse_edge_intersection_indexes(
                    historical_data.ticks.len(),
                    historical_data.candles.len(),
                    intersection,
                )),
            }
        }
    };
}

fn find_timeframe_intersection(historical_data: &HistoricalData) -> Result<Intersection> {
    let front = find_edge_intersection(historical_data, Edge::Front)?;
    let back = find_edge_intersection(historical_data, Edge::Back)?;

    Ok(Intersection { front, back })
}

/// Reduces the first candle and the first tick to the same time.
pub fn sync_candles_and_ticks(historical_data: HistoricalData) -> Result<HistoricalData> {
    if historical_data.candles.is_empty() || historical_data.ticks.is_empty() {
        bail!("empty collection of items for synchronization");
    }

    let mut trimmed_historical_data = trim_historical_data(historical_data);

    let intersection = find_timeframe_intersection(&trimmed_historical_data)?;

    let first_candle = intersection.front.candle_index;
    let last_candle = if intersection.back.candle_index > 0 {
        intersection.back.candle_index - 1
    } else {
        bail!("too little data for synchronization");
    };

    let first_tick = if intersection.front.tick_index < trimmed_historical_data.ticks.len() - 1 {
        intersection.front.tick_index + 1
    } else {
        bail!("too little data for synchronization");
    };

    let last_tick = intersection.back.tick_index;

    Ok(HistoricalData {
        candles: trimmed_historical_data
            .candles
            .drain(first_candle..=last_candle)
            .collect(),
        ticks: trimmed_historical_data
            .ticks
            .drain(first_tick..=last_tick)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use base::entities::CandleBaseProperties;

    use super::*;

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
            ],
            ticks: vec![
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
