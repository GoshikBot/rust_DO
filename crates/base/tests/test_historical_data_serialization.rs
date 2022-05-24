use base::entities::candle::BasicCandle;
use base::entities::{
    BasicTick, CandleBaseProperties, HistoricalData, StrategyProperties, Timeframe,
};
use base::historical_data::serialization;
use base::historical_data::serialization::HistoricalDataCsvSerializer;
use chrono::{DateTime, Duration, NaiveDateTime};
use tempfile::TempDir;

#[test]
fn serialize_deserialize_historical_data_proper_params_successfully() {
    let historical_data = HistoricalData {
        candles: vec![
            Some(BasicCandle {
                properties: CandleBaseProperties {
                    time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M")
                        .unwrap(),
                    ..Default::default()
                },
                edge_prices: Default::default(),
            }),
            None,
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
                time: NaiveDateTime::parse_from_str("17-05-2022 13:00", "%d-%m-%Y %H:%M").unwrap(),
                ask: 0.0,
                bid: 0.0,
            }),
            Some(BasicTick {
                time: NaiveDateTime::parse_from_str("17-05-2022 13:30", "%d-%m-%Y %H:%M").unwrap(),
                ask: 0.0,
                bid: 0.0,
            }),
            Some(BasicTick {
                time: NaiveDateTime::parse_from_str("17-05-2022 14:00", "%d-%m-%Y %H:%M").unwrap(),
                ask: 0.0,
                bid: 0.0,
            }),
            Some(BasicTick {
                time: NaiveDateTime::parse_from_str("17-05-2022 14:30", "%d-%m-%Y %H:%M").unwrap(),
                ask: 0.0,
                bid: 0.0,
            }),
            Some(BasicTick {
                time: NaiveDateTime::parse_from_str("17-05-2022 15:00", "%d-%m-%Y %H:%M").unwrap(),
                ask: 0.0,
                bid: 0.0,
            }),
            None,
            None,
            Some(BasicTick {
                time: NaiveDateTime::parse_from_str("17-05-2022 16:30", "%d-%m-%Y %H:%M").unwrap(),
                ask: 0.0,
                bid: 0.0,
            }),
        ],
    };

    let strategy_properties = StrategyProperties {
        symbol: String::from("GBPUSDm"),
        candle_timeframe: Timeframe::Hour,
        tick_timeframe: Timeframe::ThirtyMin,
        end_time: DateTime::from(
            DateTime::parse_from_str("17-05-2022 16:30 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
        ),
        duration: Duration::weeks(2),
    };

    let temp_dir = TempDir::new().unwrap();

    serialization::serialize_historical_data::<HistoricalDataCsvSerializer, _>(
        &historical_data,
        &strategy_properties,
        temp_dir.path(),
    )
    .unwrap();

    let deserialized_historical_data = serialization::try_to_deserialize_historical_data::<
        HistoricalDataCsvSerializer,
        _,
    >(&strategy_properties, temp_dir.path())
    .unwrap()
    .unwrap();

    assert_eq!(deserialized_historical_data, historical_data);
}
