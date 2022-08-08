use backtesting::historical_data::serialization::{
    HistoricalDataCsvSerialization, HistoricalDataSerialization,
};
use backtesting::{HistoricalData, StrategyInitConfig};
use base::entities::candle::BasicCandleProperties;
use base::entities::{BasicTickProperties, StrategyTimeframes, Timeframe};
use chrono::{DateTime, Duration, NaiveDateTime};
use rust_decimal_macros::dec;
use tempfile::TempDir;

#[test]
fn serialize_deserialize_historical_data_proper_params_successfully() {
    let historical_data = HistoricalData {
        candles: vec![
            Some(BasicCandleProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M").unwrap(),
                ..Default::default()
            }),
            None,
            Some(BasicCandleProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M").unwrap(),
                ..Default::default()
            }),
        ],
        ticks: vec![
            Some(BasicTickProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M").unwrap(),
                ask: dec!(0.0),
                bid: dec!(0.0),
            }),
            Some(BasicTickProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M").unwrap(),
                ask: dec!(0.0),
                bid: dec!(0.0),
            }),
            Some(BasicTickProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M").unwrap(),
                ask: dec!(0.0),
                bid: dec!(0.0),
            }),
            Some(BasicTickProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 14:30", "%d-%m-%Y %H:%M").unwrap(),
                ask: dec!(0.0),
                bid: dec!(0.0),
            }),
            Some(BasicTickProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M").unwrap(),
                ask: dec!(0.0),
                bid: dec!(0.0),
            }),
            None,
            None,
            Some(BasicTickProperties {
                time: NaiveDateTime::parse_from_str("17-05-2022 16:30", "%d-%m-%Y %H:%M").unwrap(),
                ask: dec!(0.0),
                bid: dec!(0.0),
            }),
        ],
    };

    let strategy_properties = StrategyInitConfig {
        symbol: String::from("GBPUSDm"),
        timeframes: StrategyTimeframes {
            candle: Timeframe::Hour,
            tick: Timeframe::ThirtyMin,
        },
        end_time: DateTime::from(
            DateTime::parse_from_str("17-05-2022 16:30 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
        ),
        duration: Duration::weeks(2),
    };

    let temp_dir = TempDir::new().unwrap();

    let historical_data_csv_serialization = HistoricalDataCsvSerialization::new();

    historical_data_csv_serialization
        .serialize_historical_data(&historical_data, &strategy_properties, temp_dir.path())
        .unwrap();

    let expected_candles_file_path = temp_dir
        .path()
        .join(r"GBPUSDm_1h_30m_2022-05-17_16-30_20160_(2_weeks)/candles.csv");
    let expected_ticks_file_path = temp_dir
        .path()
        .join(r"GBPUSDm_1h_30m_2022-05-17_16-30_20160_(2_weeks)/ticks.csv");

    assert!(expected_candles_file_path.exists());
    assert!(expected_ticks_file_path.exists());

    let deserialized_historical_data = historical_data_csv_serialization
        .try_to_deserialize_historical_data(&strategy_properties, temp_dir.path())
        .unwrap()
        .unwrap();

    assert_eq!(deserialized_historical_data, historical_data);
}
