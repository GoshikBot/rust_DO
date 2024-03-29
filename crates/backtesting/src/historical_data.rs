use crate::historical_data::serialization::HistoricalDataSerialization;
use crate::{HistoricalData, StrategyInitConfig};
use anyhow::{Context, Result};
use base::entities::candle::BasicCandleProperties;
use base::entities::tick::{HistoricalTickPrice, TickPrice};
use base::entities::{BasicTickProperties, StrategyTimeframes};
use std::path::PathBuf;
use trading_apis::MarketDataApi;

pub mod serialization;
pub mod synchronization;

/// Tries to deserialize historical data if it exists. Otherwise, requests a market data api
/// and serializes the got data for caching purposes.
pub fn get_historical_data<S, M, P>(
    historical_data_folder: P,
    strategy_properties: &StrategyInitConfig,
    market_data_api: &M,
    serialization: &S,
    sync_candles_and_ticks: impl Fn(
        HistoricalData<BasicCandleProperties, BasicTickProperties<HistoricalTickPrice>>,
    ) -> Result<
        HistoricalData<BasicCandleProperties, BasicTickProperties<HistoricalTickPrice>>,
    >,
) -> Result<HistoricalData<BasicCandleProperties, BasicTickProperties<HistoricalTickPrice>>>
where
    S: HistoricalDataSerialization,
    M: MarketDataApi<
        CandleProperties = BasicCandleProperties,
        RealTickProperties = BasicTickProperties<TickPrice>,
        HistoricalTickProperties = BasicTickProperties<HistoricalTickPrice>,
    >,
    P: Into<PathBuf> + Clone,
{
    let StrategyInitConfig {
        symbol,
        timeframes:
            StrategyTimeframes {
                candle: candle_timeframe,
                tick: tick_timeframe,
            },
        end_time,
        duration,
    } = strategy_properties;

    let historical_data = serialization
        .try_to_deserialize_historical_data(strategy_properties, historical_data_folder.clone())?;

    let historical_data = match historical_data {
        Some(historical_data) => historical_data,
        None => {
            let candles = market_data_api.get_historical_candles(
                symbol,
                *candle_timeframe,
                *end_time,
                *duration,
            )?;
            let ticks = market_data_api.get_historical_ticks(
                symbol,
                *tick_timeframe,
                *end_time,
                *duration,
            )?;

            let historical_data = sync_candles_and_ticks(HistoricalData { candles, ticks })
                .context("error on synchronizing ticks and candles")?;

            serialization.serialize_historical_data(
                &historical_data,
                strategy_properties,
                historical_data_folder,
            )?;

            historical_data
        }
    };

    Ok(historical_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::entities::candle::BasicCandleProperties;
    use base::entities::{BasicTickProperties, Timeframe};
    use chrono::{DateTime, Duration, NaiveDateTime, Utc};
    use std::cell::RefCell;

    struct MarketDataTestApi;

    impl MarketDataApi for MarketDataTestApi {
        type RealTickProperties = BasicTickProperties<TickPrice>;
        type HistoricalTickProperties = BasicTickProperties<HistoricalTickPrice>;
        type CandleProperties = BasicCandleProperties;

        fn get_current_tick(&self, _symbol: &str) -> Result<Self::RealTickProperties> {
            todo!()
        }

        fn get_current_candle(
            &self,
            _symbol: &str,
            _timeframe: Timeframe,
        ) -> Result<Self::CandleProperties> {
            todo!()
        }

        fn get_historical_candles(
            &self,
            _symbol: &str,
            _timeframe: Timeframe,
            _end_time: DateTime<Utc>,
            _duration: Duration,
        ) -> Result<Vec<Option<Self::CandleProperties>>> {
            Ok(vec![
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 18:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                None,
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 19:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
            ])
        }

        fn get_historical_ticks(
            &self,
            _symbol: &str,
            _timeframe: Timeframe,
            _end_time: DateTime<Utc>,
            _duration: Duration,
        ) -> Result<Vec<Option<Self::HistoricalTickProperties>>> {
            Ok(vec![
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 18:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 18:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
            ])
        }
    }

    #[derive(Default)]
    struct HistoricalDataTestSerializationDataExists {
        serialization_is_called: RefCell<bool>,
        deserialization_is_called: RefCell<bool>,
    }

    impl HistoricalDataSerialization for HistoricalDataTestSerializationDataExists {
        fn serialize_historical_data<P: Into<PathBuf>>(
            &self,
            _historical_data: &HistoricalData<
                BasicCandleProperties,
                BasicTickProperties<HistoricalTickPrice>,
            >,
            _strategy_properties: &StrategyInitConfig,
            _directory: P,
        ) -> Result<()> {
            *self.serialization_is_called.borrow_mut() = true;
            Ok(())
        }

        fn try_to_deserialize_historical_data<P: Into<PathBuf>>(
            &self,
            _strategy_properties: &StrategyInitConfig,
            _directory: P,
        ) -> Result<
            Option<HistoricalData<BasicCandleProperties, BasicTickProperties<HistoricalTickPrice>>>,
        > {
            *self.deserialization_is_called.borrow_mut() = true;

            Ok(Some(HistoricalData {
                candles: vec![
                    Some(BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    }),
                    None,
                    Some(BasicCandleProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    }),
                ],
                ticks: vec![
                    Some(BasicTickProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    }),
                    Some(BasicTickProperties {
                        time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M")
                            .unwrap(),
                        ..Default::default()
                    }),
                ],
            }))
        }
    }

    #[derive(Default)]
    struct HistoricalDataTestSerializationDataDoesNotExist {
        serialization_is_called: RefCell<bool>,
        deserialization_is_called: RefCell<bool>,
    }

    impl HistoricalDataSerialization for HistoricalDataTestSerializationDataDoesNotExist {
        fn serialize_historical_data<P: Into<PathBuf>>(
            &self,
            _historical_data: &HistoricalData<
                BasicCandleProperties,
                BasicTickProperties<HistoricalTickPrice>,
            >,
            _strategy_properties: &StrategyInitConfig,
            _directory: P,
        ) -> Result<()> {
            *self.serialization_is_called.borrow_mut() = true;

            Ok(())
        }

        fn try_to_deserialize_historical_data<P: Into<PathBuf>>(
            &self,
            _strategy_properties: &StrategyInitConfig,
            _directory: P,
        ) -> Result<
            Option<HistoricalData<BasicCandleProperties, BasicTickProperties<HistoricalTickPrice>>>,
        > {
            *self.deserialization_is_called.borrow_mut() = true;
            Ok(None)
        }
    }

    #[test]
    fn get_historical_data_already_exists_successfully_deserialize() {
        let strategy_properties = StrategyInitConfig {
            symbol: String::from("GBPUSDm"),
            timeframes: StrategyTimeframes {
                candle: Timeframe::Hour,
                tick: Timeframe::OneMin,
            },
            end_time: DateTime::from(
                DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
            ),
            duration: Duration::weeks(16),
        };

        let historical_data_serialization: HistoricalDataTestSerializationDataExists =
            Default::default();

        let market_data_api = MarketDataTestApi {};

        let expected_historical_data = HistoricalData {
            candles: vec![
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                None,
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
            ],
            ticks: vec![
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
            ],
        };

        let sync_candles_and_ticks_is_called = RefCell::new(false);
        let sync_candles_and_ticks = |historical_data: HistoricalData<
            BasicCandleProperties,
            BasicTickProperties<HistoricalTickPrice>,
        >| {
            *sync_candles_and_ticks_is_called.borrow_mut() = true;
            Ok(historical_data)
        };

        let historical_data = get_historical_data(
            "test",
            &strategy_properties,
            &market_data_api,
            &historical_data_serialization,
            sync_candles_and_ticks,
        )
        .unwrap();

        assert_eq!(historical_data, expected_historical_data);

        assert!(*historical_data_serialization
            .deserialization_is_called
            .borrow());
        assert!(!*sync_candles_and_ticks_is_called.borrow());
        assert!(!*historical_data_serialization
            .serialization_is_called
            .borrow());
    }

    #[test]
    fn get_historical_data_does_not_exists_successfully_got_and_serialize() {
        let strategy_properties = StrategyInitConfig {
            symbol: String::from("GBPUSDm"),
            timeframes: StrategyTimeframes {
                candle: Timeframe::Hour,
                tick: Timeframe::OneMin,
            },
            end_time: DateTime::from(
                DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
            ),
            duration: Duration::weeks(16),
        };

        let historical_data_serialization: HistoricalDataTestSerializationDataDoesNotExist =
            Default::default();

        let market_data_api = MarketDataTestApi {};

        let expected_historical_data = HistoricalData {
            candles: vec![
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 18:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                None,
                Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 19:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
            ],
            ticks: vec![
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 18:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
                Some(BasicTickProperties {
                    time: NaiveDateTime::parse_from_str("19-05-2022 18:30", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                }),
            ],
        };

        let sync_candles_and_ticks_is_called = RefCell::new(false);
        let sync_candles_and_ticks = |historical_data: HistoricalData<
            BasicCandleProperties,
            BasicTickProperties<HistoricalTickPrice>,
        >| {
            *sync_candles_and_ticks_is_called.borrow_mut() = true;
            Ok(historical_data)
        };

        let historical_data = get_historical_data(
            "test",
            &strategy_properties,
            &market_data_api,
            &historical_data_serialization,
            sync_candles_and_ticks,
        )
        .unwrap();

        assert_eq!(historical_data, expected_historical_data);

        assert!(*historical_data_serialization
            .deserialization_is_called
            .borrow());
        assert!(*sync_candles_and_ticks_is_called.borrow());
        assert!(*historical_data_serialization
            .serialization_is_called
            .borrow());
    }
}
