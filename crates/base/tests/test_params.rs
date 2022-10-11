use base::params::{StrategyMultiSourcingParams, StrategyParam, StrategyParams};
use csv::Writer;
use rust_decimal_macros::dec;
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
fn should_get_params_from_csv_file_successfully() {
    let dir = tempfile::tempdir().unwrap();

    let params = vec![
        StrategyParam {
            name: String::from("max_distance_from_corridor_leading_candle_pins_pct"),
            value: String::from("12.5"),
        },
        StrategyParam {
            name: String::from("min_distance_between_max_min_angles"),
            value: String::from("4.3k"),
        },
    ];

    let settings_file_path = dir.path().join("settings.csv");

    let mut writer = Writer::from_path(&settings_file_path).unwrap();
    for setting in params {
        writer.serialize(setting).unwrap();
    }
    writer.flush().unwrap();

    let params: StrategyMultiSourcingParams<PointParam, RatioParam> =
        StrategyMultiSourcingParams::from_csv(&settings_file_path).unwrap();

    assert_eq!(
        params.get_point_param_value(PointParam::MaxDistanceFromCorridorLeadingCandlePinsPct),
        dec!(12.5)
    );

    assert_eq!(
        params.get_ratio_param_value(RatioParam::MinDistanceBetweenMaxMinAngles, 10),
        dec!(43.0)
    )
}

#[test]
fn should_get_params_from_vec_successfully() {
    let params = vec![
        StrategyParam {
            name: String::from("max_distance_from_corridor_leading_candle_pins_pct"),
            value: String::from("12.5"),
        },
        StrategyParam {
            name: String::from("min_distance_between_max_min_angles"),
            value: String::from("4.3k"),
        },
    ];

    let params: StrategyMultiSourcingParams<PointParam, RatioParam> =
        StrategyMultiSourcingParams::from_vec(params).unwrap();

    assert_eq!(
        params.get_point_param_value(PointParam::MaxDistanceFromCorridorLeadingCandlePinsPct),
        dec!(12.5)
    );

    assert_eq!(
        params.get_ratio_param_value(RatioParam::MinDistanceBetweenMaxMinAngles, 10),
        dec!(43.0)
    )
}
