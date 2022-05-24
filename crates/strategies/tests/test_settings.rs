use csv::Writer;
use std::fmt::{Display, Formatter};
use strategies::step::utils::settings::{CsvFileSettings, CsvSetting, Settings};

enum PointSetting {
    MaxDistanceFromCorridorLeadingCandlePinsPct,
}

impl Display for PointSetting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            PointSetting::MaxDistanceFromCorridorLeadingCandlePinsPct => {
                write!(f, "max_distance_from_corridor_leading_candle_pins_pct")
            }
        }
    }
}

enum RatioSetting {
    MinDistanceBetweenMaxMinAngles,
}

impl Display for RatioSetting {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            RatioSetting::MinDistanceBetweenMaxMinAngles => {
                write!(f, "min_distance_between_max_min_angles")
            }
        }
    }
}

#[test]
fn get_point_and_ratio_settings_existing_settings_successfully() {
    let dir = tempfile::tempdir().unwrap();

    let settings = vec![
        CsvSetting {
            name: String::from("max_distance_from_corridor_leading_candle_pins_pct"),
            value: String::from("12.5"),
        },
        CsvSetting {
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

    let settings: CsvFileSettings<PointSetting, RatioSetting> =
        CsvFileSettings::new(&settings_file_path).unwrap();

    assert_eq!(
        settings.get_point_setting_value(PointSetting::MaxDistanceFromCorridorLeadingCandlePinsPct),
        12.5
    );

    assert_eq!(
        settings.get_ratio_setting_value(RatioSetting::MinDistanceBetweenMaxMinAngles, 10.0),
        43.0
    )
}
