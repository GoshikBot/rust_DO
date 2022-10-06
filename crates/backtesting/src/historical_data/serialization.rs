use crate::{get_path_name_for_data_config, HistoricalData, StrategyInitConfig};
use base::entities::candle::{BasicCandleProperties, CandlePrice, CandleSize, CandleVolatility};
use base::entities::tick::TickPrice;
use base::entities::{BasicTickProperties, CandlePrices, CandleType, StrategyTimeframes};
use chrono::NaiveDateTime;
use csv::{Reader, Writer};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

const TIME_PATTERN_FOR_SERIALIZATION: &str = "%Y-%m-%d %H:%M";

const CANDLES_CSV_FILE_NAME: &str = "candles.csv";
const TICKS_CSV_FILE_NAME: &str = "ticks.csv";

#[derive(Debug, Eq, PartialEq)]
pub struct HistoricalDataPaths {
    candles_file_path: PathBuf,
    ticks_file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Candle {
    time: Option<String>,
    r#type: Option<CandleType>,
    size: Option<CandleSize>,
    volatility: Option<CandleVolatility>,
    open: Option<CandlePrice>,
    high: Option<CandlePrice>,
    low: Option<CandlePrice>,
    close: Option<CandlePrice>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Tick {
    time: Option<String>,
    ask: Option<TickPrice>,
    bid: Option<TickPrice>,
}

fn get_paths_for_historical_data<P: Into<PathBuf>>(
    directory: P,
    strategy_config: &StrategyInitConfig,
) -> HistoricalDataPaths {
    let mut directory = directory.into();

    let directory_for_candles_and_ticks = get_path_name_for_data_config(strategy_config);
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

fn historical_data_files_exist(historical_data_paths: &HistoricalDataPaths) -> bool {
    historical_data_paths.candles_file_path.exists()
        && historical_data_paths.ticks_file_path.exists()
}

pub trait HistoricalDataSerialization {
    fn serialize_historical_data<P: Into<PathBuf>>(
        &self,
        historical_data: &HistoricalData<BasicCandleProperties, BasicTickProperties>,
        strategy_config: &StrategyInitConfig,
        directory: P,
    ) -> anyhow::Result<()>;

    fn try_to_deserialize_historical_data<P: Into<PathBuf>>(
        &self,
        strategy_config: &StrategyInitConfig,
        directory: P,
    ) -> anyhow::Result<Option<HistoricalData<BasicCandleProperties, BasicTickProperties>>>;
}

#[derive(Default)]
pub struct HistoricalDataCsvSerialization;

impl HistoricalDataCsvSerialization {
    pub fn new() -> Self {
        Default::default()
    }

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

    fn serialize(
        historical_data: &HistoricalData<BasicCandleProperties, BasicTickProperties>,
        historical_data_paths: &HistoricalDataPaths,
    ) -> anyhow::Result<()> {
        let HistoricalDataPaths {
            candles_file_path,
            ticks_file_path,
        } = historical_data_paths;

        Self::create_required_dirs_for_serialization(historical_data_paths)?;

        let mut candles_writer = Writer::from_path(candles_file_path)?;

        for candle in historical_data.candles.iter() {
            let serializable_candle = match candle.as_ref() {
                Some(candle) => Candle {
                    time: Some(
                        candle
                            .time
                            .format(TIME_PATTERN_FOR_SERIALIZATION)
                            .to_string(),
                    ),
                    r#type: Some(candle.r#type),
                    size: Some(candle.size),
                    volatility: Some(candle.volatility),
                    open: Some(candle.prices.open),
                    high: Some(candle.prices.high),
                    low: Some(candle.prices.low),
                    close: Some(candle.prices.close),
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

    fn deserialize(
        historical_data_paths: &HistoricalDataPaths,
    ) -> anyhow::Result<HistoricalData<BasicCandleProperties, BasicTickProperties>> {
        let HistoricalDataPaths {
            candles_file_path,
            ticks_file_path,
        } = historical_data_paths;

        let mut candles: Vec<Option<BasicCandleProperties>> = Vec::new();
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
                } => candles.push(Some(BasicCandleProperties {
                    time: NaiveDateTime::parse_from_str(&time, TIME_PATTERN_FOR_SERIALIZATION)?,
                    r#type,
                    size,
                    volatility,
                    prices: CandlePrices {
                        open,
                        high,
                        low,
                        close,
                    },
                })),
                _ => candles.push(None),
            }
        }

        let mut ticks: Vec<Option<BasicTickProperties>> = Vec::new();
        let mut ticks_reader = Reader::from_path(ticks_file_path)?;

        for tick in ticks_reader.deserialize() {
            let tick: Tick = tick?;

            match tick {
                Tick {
                    time: Some(time),
                    ask: Some(ask),
                    bid: Some(bid),
                } => ticks.push(Some(BasicTickProperties {
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

impl HistoricalDataSerialization for HistoricalDataCsvSerialization {
    fn serialize_historical_data<P: Into<PathBuf>>(
        &self,
        historical_data: &HistoricalData<BasicCandleProperties, BasicTickProperties>,
        strategy_config: &StrategyInitConfig,
        directory: P,
    ) -> anyhow::Result<()> {
        let historical_data_paths = get_paths_for_historical_data(directory, strategy_config);
        Self::serialize(historical_data, &historical_data_paths)
    }

    fn try_to_deserialize_historical_data<P: Into<PathBuf>>(
        &self,
        strategy_config: &StrategyInitConfig,
        directory: P,
    ) -> anyhow::Result<Option<HistoricalData<BasicCandleProperties, BasicTickProperties>>> {
        let historical_data_paths = get_paths_for_historical_data(directory, strategy_config);

        if historical_data_files_exist(&historical_data_paths) {
            let historical_data = Self::deserialize(&historical_data_paths)?;
            Ok(Some(historical_data))
        } else {
            Ok(None)
        }
    }
}
