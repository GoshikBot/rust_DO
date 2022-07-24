use base::params::{StrategyCsvFileParams, StrategyCsvParam, StrategyParams};
use csv::Writer;
use std::fmt::{Display, Formatter};

enum PointParam {
    MaxDistanceFromCorridorLeadingCandlePinsPct,
}

impl Display for PointParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            PointParam::MaxDistanceFromCorridorLeadingCandlePinsPct => {
                write!(f, "max_distance_from_corridor_leading_candle_pins_pct")
            }
        }
    }
}

enum RatioParam {
    MinDistanceBetweenMaxMinAngles,
}

impl Display for RatioParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            RatioParam::MinDistanceBetweenMaxMinAngles => {
                write!(f, "min_distance_between_max_min_angles")
            }
        }
    }
}

#[test]
fn get_point_and_ratio_settings_existing_settings_successfully() {
    let dir = tempfile::tempdir().unwrap();

    let settings = vec![
        StrategyCsvParam {
            name: String::from("max_distance_from_corridor_leading_candle_pins_pct"),
            value: String::from("12.5"),
        },
        StrategyCsvParam {
            name: String::from("min_distance_between_max_min_angles"),
            value: String::from("4.3k"),
        },
    ];

    let settings_file_path = dir.path().join("settings.csv");

    let mut writer = Writer::from_path(&settings_file_path).unwrap();
    for setting in settings {
        writer.serialize(setting).unwrap();
    }
    writer.flush().unwrap();

    let settings: StrategyCsvFileParams<PointParam, RatioParam> =
        StrategyCsvFileParams::new(&settings_file_path).unwrap();

    assert_eq!(
        settings.get_point_param_value(PointParam::MaxDistanceFromCorridorLeadingCandlePinsPct),
        12.5
    );

    assert_eq!(
        settings.get_ratio_param_value(RatioParam::MinDistanceBetweenMaxMinAngles, 10.0),
        43.0
    )
}
