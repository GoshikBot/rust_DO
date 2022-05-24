use crate::entities::candle::{BasicCandle, CandleEdgePrice, CandleSize, CandleVolatility};
use crate::entities::tick::TickPrice;
use crate::entities::{
    BasicTick, CandleBaseProperties, CandleEdgePrices, CandleType, HistoricalData,
    StrategyProperties,
};
use chrono::NaiveDateTime;
use csv::{Reader, Writer};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

const TIME_PATTERN_FOR_SERIALIZATION: &str = "%Y-%m-%d %H:%M";
const TIME_PATTERN_FOR_PATH: &str = "%Y-%m-%d_%H-%M";

const CANDLES_CSV_FILE_NAME: &str = "candles.csv";
const TICKS_CSV_FILE_NAME: &str = "ticks.csv";

#[derive(Debug, Eq, PartialEq)]
pub struct HistoricalDataPaths {
    candles_file_path: PathBuf,
    ticks_file_path: PathBuf,
}

pub trait HistoricalDataSerializer {
    fn serialize(
        historical_data: &HistoricalData,
        historical_data_paths: &HistoricalDataPaths,
    ) -> anyhow::Result<()>;

    fn deserialize(historical_data_paths: &HistoricalDataPaths) -> anyhow::Result<HistoricalData>;
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Candle {
    time: Option<String>,
    r#type: Option<CandleType>,
    size: Option<CandleSize>,
    volatility: Option<CandleVolatility>,
    open: Option<CandleEdgePrice>,
    high: Option<CandleEdgePrice>,
    low: Option<CandleEdgePrice>,
    close: Option<CandleEdgePrice>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Tick {
    time: Option<String>,
    ask: Option<TickPrice>,
    bid: Option<TickPrice>,
}

pub struct HistoricalDataCsvSerializer;

impl HistoricalDataCsvSerializer {
    fn create_required_dirs_for_serialization(
        historical_data_paths: &HistoricalDataPaths,
    ) -> anyhow::Result<()> {
        let HistoricalDataPaths {
            ticks_file_path,
            candles_file_path,
        } = historical_data_paths;

        match (
            ticks_file_path.extension().and_then(OsStr::to_str),
            candles_file_path.extension().and_then(OsStr::to_str),
        ) {
            (Some("csv"), Some("csv")) => {
                let ticks_path_without_file_name = ticks_file_path.parent();

                if let Some(ticks_path_without_file_name) = ticks_path_without_file_name {
                    fs::create_dir_all(ticks_path_without_file_name)?;
                }
            }
            _ => anyhow::bail!("wrong csv file paths: {:?}", historical_data_paths),
        }

        Ok(())
    }
}

impl HistoricalDataSerializer for HistoricalDataCsvSerializer {
    fn serialize(
        historical_data: &HistoricalData,
        historical_data_paths: &HistoricalDataPaths,
    ) -> anyhow::Result<()> {
        let HistoricalDataPaths {
            candles_file_path,
            ticks_file_path,
        } = historical_data_paths;

        HistoricalDataCsvSerializer::create_required_dirs_for_serialization(historical_data_paths)?;

        let mut candles_writer = Writer::from_path(candles_file_path)?;

        for candle in historical_data.candles.iter() {
            let serializable_candle = match candle.as_ref() {
                Some(candle) => Candle {
                    time: Some(
                        candle
                            .properties
                            .time
                            .format(TIME_PATTERN_FOR_SERIALIZATION)
                            .to_string(),
                    ),
                    r#type: Some(candle.properties.r#type),
                    size: Some(candle.properties.size),
                    volatility: Some(candle.properties.volatility),
                    open: Some(candle.edge_prices.open),
                    high: Some(candle.edge_prices.high),
                    low: Some(candle.edge_prices.low),
                    close: Some(candle.edge_prices.close),
                },
                None => Default::default(),
            };

            candles_writer.serialize(serializable_candle)?;
        }

        let mut ticks_writer = Writer::from_path(ticks_file_path)?;

        for tick in historical_data.ticks.iter() {
            let serializable_tick = match tick.as_ref() {
                Some(tick) => Tick {
                    time: Some(tick.time.format(TIME_PATTERN_FOR_SERIALIZATION).to_string()),
                    ask: Some(tick.ask),
                    bid: Some(tick.bid),
                },
                None => Default::default(),
            };

            ticks_writer.serialize(serializable_tick)?;
        }

        Ok(())
    }

    fn deserialize(historical_data_paths: &HistoricalDataPaths) -> anyhow::Result<HistoricalData> {
        let HistoricalDataPaths {
            candles_file_path,
            ticks_file_path,
        } = historical_data_paths;

        let mut candles: Vec<Option<BasicCandle>> = Vec::new();
        let mut candles_reader = Reader::from_path(candles_file_path)?;

        for candle in candles_reader.deserialize() {
            let candle: Candle = candle?;
            match candle {
                Candle {
                    time: Some(time),
                    r#type: Some(r#type),
                    size: Some(size),
                    volatility: Some(volatility),
                    open: Some(open),
                    high: Some(high),
                    low: Some(low),
                    close: Some(close),
                } => candles.push(Some(BasicCandle {
                    properties: CandleBaseProperties {
                        time: NaiveDateTime::parse_from_str(&time, TIME_PATTERN_FOR_SERIALIZATION)?,
                        r#type,
                        size,
                        volatility,
                    },
                    edge_prices: CandleEdgePrices {
                        open,
                        high,
                        low,
                        close,
                    },
                })),
                _ => candles.push(None),
            }
        }

        let mut ticks: Vec<Option<BasicTick>> = Vec::new();
        let mut ticks_reader = Reader::from_path(ticks_file_path)?;

        for tick in ticks_reader.deserialize() {
            let tick: Tick = tick?;

            match tick {
                Tick {
                    time: Some(time),
                    ask: Some(ask),
                    bid: Some(bid),
                } => ticks.push(Some(BasicTick {
                    time: NaiveDateTime::parse_from_str(&time, TIME_PATTERN_FOR_SERIALIZATION)?,
                    ask,
                    bid,
                })),
                _ => ticks.push(None),
            }
        }

        Ok(HistoricalData { candles, ticks })
    }
}

fn get_directory_name_for_data_config(strategy_properties: &StrategyProperties) -> String {
    let StrategyProperties {
        symbol,
        candle_timeframe,
        tick_timeframe,
        end_time,
        duration,
    } = strategy_properties;

    format!(
        "{}_{}_{}_{}_{}_({}_weeks)",
        symbol,
        candle_timeframe,
        tick_timeframe,
        end_time.format(TIME_PATTERN_FOR_PATH),
        duration.num_minutes(),
        duration.num_weeks()
    )
}

fn get_paths_for_historical_data<P: Into<PathBuf>>(
    directory: P,
    strategy_properties: &StrategyProperties,
) -> HistoricalDataPaths {
    let mut directory = directory.into();

    let directory_for_candles_and_ticks = get_directory_name_for_data_config(strategy_properties);
    directory.push(directory_for_candles_and_ticks);

    let mut candles_file_path = directory.clone();
    candles_file_path.push(CANDLES_CSV_FILE_NAME);

    let mut ticks_file_path = directory;
    ticks_file_path.push(TICKS_CSV_FILE_NAME);

    HistoricalDataPaths {
        candles_file_path,
        ticks_file_path,
    }
}

pub fn serialize_historical_data<T, P>(
    historical_data: &HistoricalData,
    strategy_properties: &StrategyProperties,
    directory: P,
) -> anyhow::Result<()>
where
    T: HistoricalDataSerializer,
    P: Into<PathBuf>,
{
    let historical_data_paths = get_paths_for_historical_data(directory, strategy_properties);
    T::serialize(historical_data, &historical_data_paths)
}

fn historical_data_files_exist(historical_data_paths: &HistoricalDataPaths) -> bool {
    historical_data_paths.candles_file_path.exists()
        && historical_data_paths.ticks_file_path.exists()
}

pub fn try_to_deserialize_historical_data<T, P>(
    strategy_properties: &StrategyProperties,
    directory: P,
) -> anyhow::Result<Option<HistoricalData>>
where
    T: HistoricalDataSerializer,
    P: Into<PathBuf>,
{
    let historical_data_paths = get_paths_for_historical_data(directory, strategy_properties);

    if historical_data_files_exist(&historical_data_paths) {
        let historical_data = T::deserialize(&historical_data_paths)?;
        Ok(Some(historical_data))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Timeframe;
    use chrono::{DateTime, Duration};

    struct TestHistoricalDataSerializer;

    impl TestHistoricalDataSerializer {
        fn expected_paths() -> HistoricalDataPaths {
            HistoricalDataPaths {
                candles_file_path: PathBuf::from(
                    r"D:\GBPUSDm_1h_1m_2022-05-17_18-00_20160_(2_weeks)\candles.csv",
                ),
                ticks_file_path: PathBuf::from(
                    r"D:\GBPUSDm_1h_1m_2022-05-17_18-00_20160_(2_weeks)\ticks.csv",
                ),
            }
        }
    }

    impl HistoricalDataSerializer for TestHistoricalDataSerializer {
        fn serialize(
            _historical_data: &HistoricalData,
            historical_data_paths: &HistoricalDataPaths,
        ) -> anyhow::Result<()> {
            assert_eq!(
                historical_data_paths,
                &TestHistoricalDataSerializer::expected_paths()
            );

            Ok(())
        }

        fn deserialize(
            historical_data_paths: &HistoricalDataPaths,
        ) -> anyhow::Result<HistoricalData> {
            assert_eq!(
                historical_data_paths,
                &TestHistoricalDataSerializer::expected_paths()
            );

            Ok(Default::default())
        }
    }

    #[test]
    fn serialize_historical_data_proper_paths_true() {
        let strategy_properties = StrategyProperties {
            symbol: String::from("GBPUSDm"),
            candle_timeframe: Timeframe::Hour,
            tick_timeframe: Timeframe::OneMin,
            end_time: DateTime::from(
                DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
            ),
            duration: Duration::weeks(2),
        };

        let directory = r"D:\";

        let _ = serialize_historical_data::<TestHistoricalDataSerializer, _>(
            &Default::default(),
            &strategy_properties,
            directory,
        );
    }

    #[test]
    fn try_to_deserialize_historical_data_proper_paths_true() {
        let strategy_properties = StrategyProperties {
            symbol: String::from("GBPUSDm"),
            candle_timeframe: Timeframe::Hour,
            tick_timeframe: Timeframe::OneMin,
            end_time: DateTime::from(
                DateTime::parse_from_str("17-05-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
            ),
            duration: Duration::weeks(2),
        };

        let directory = r"D:\";

        let _ = try_to_deserialize_historical_data::<TestHistoricalDataSerializer, _>(
            &strategy_properties,
            directory,
        );
    }
}
