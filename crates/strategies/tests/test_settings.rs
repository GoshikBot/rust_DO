use strategies::step::utils::settings::{ExcelFileSettings, Settings};

#[test]
fn test_getting_point_setting_value() {
    let settings =
        ExcelFileSettings::new(r"D:\work_and_projects\Goshik_bot\rust_bot_DO\step_settings.csv")
            .unwrap();

    assert_eq!(
        settings
            .get_point_setting_value("max_distance_from_corridor_leading_candle_pins_pct")
            .unwrap(),
        27.993_973
    );
}

#[test]
fn test_getting_ratio_setting_value() {
    let settings =
        ExcelFileSettings::new(r"D:\work_and_projects\Goshik_bot\rust_bot_DO\step_settings.csv")
            .unwrap();

    assert_eq!(
        settings
            .get_ratio_setting_value("min_distance_between_max_min_angles", 10.0)
            .unwrap(),
        8.124_612
    );
}
