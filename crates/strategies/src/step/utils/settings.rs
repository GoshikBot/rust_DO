use std::collections::HashMap;

use csv::Reader;
use serde::Deserialize;
use simple_error::{map_err_with, require_with, try_with, SimpleResult};

use base::entities::candle::CandleVolatility;

pub type SettingName = String;
pub type SettingValue = String;
pub type PointSettingValue = f32;

fn get_point_value_from_ratio(
    ratio_value: &str,
    volatility: CandleVolatility,
) -> SimpleResult<PointSettingValue> {
    Ok(try_with!(
        ratio_value[..ratio_value.len() - 1].parse::<f32>(),
        "an error on parsing a ratio value {}",
        ratio_value
    ) * volatility)
}

pub trait Settings {
    fn get_point_setting_value(&self, name: &str) -> SimpleResult<PointSettingValue>;

    fn get_ratio_setting_value(
        &self,
        name: &str,
        volatility: CandleVolatility,
    ) -> SimpleResult<PointSettingValue>;
}

#[derive(Debug, Deserialize)]
struct CsvRecord {
    setting_name: SettingName,
    setting_value: SettingValue,
}

pub struct ExcelFileSettings {
    names_values: HashMap<SettingName, SettingValue>,
}

impl ExcelFileSettings {
    pub fn new(path_to_file: &str) -> SimpleResult<Self> {
        let mut settings = Self {
            names_values: Default::default(),
        };

        let mut reader = try_with!(
            Reader::from_path(path_to_file),
            "an error on create a reader from a path"
        );

        for record in reader.deserialize::<CsvRecord>() {
            let record = try_with!(record, "an error on deserializing a record");
            settings
                .names_values
                .insert(record.setting_name, record.setting_value);
        }

        Ok(settings)
    }
}

impl Settings for ExcelFileSettings {
    fn get_point_setting_value(&self, name: &str) -> SimpleResult<PointSettingValue> {
        let value = require_with!(
            self.names_values.get(name),
            "a point setting with a name {} is not found",
            name
        );

        map_err_with!(
            value.parse::<PointSettingValue>(),
            "an error on parsing a point setting with a name {}",
            name
        )
    }

    fn get_ratio_setting_value(
        &self,
        name: &str,
        volatility: CandleVolatility,
    ) -> SimpleResult<PointSettingValue> {
        let ratio_value = require_with!(
            self.names_values.get(name),
            "a ratio setting with a name {} is not found",
            name
        );

        map_err_with!(
            get_point_value_from_ratio(ratio_value, volatility),
            "an error occurred during a conversion from a ratio value {} to a point value",
            name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successfully_get_point_value_from_ratio() {
        assert_eq!(get_point_value_from_ratio("1.234k", 10.0).unwrap(), 12.34);
    }

    #[test]
    fn unsuccessfully_get_point_value_from_ratio() {
        assert!(get_point_value_from_ratio("hello", 10.0).is_err());
    }
}
