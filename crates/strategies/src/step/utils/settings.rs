use anyhow::Context;
use std::collections::HashMap;
use std::num::ParseFloatError;

use csv::Reader;
use serde::Deserialize;

use base::entities::candle::CandleVolatility;

pub type SettingName = String;
pub type SettingValue = String;
pub type PointSettingValue = f32;

fn get_point_value_from_ratio(
    ratio_value: &str,
    volatility: CandleVolatility,
) -> Result<f32, ParseFloatError> {
    Ok(ratio_value[..ratio_value.len() - 1].parse::<f32>()? * volatility)
}

pub trait Settings {
    fn get_point_setting_value(&self, name: &str) -> anyhow::Result<PointSettingValue>;

    fn get_ratio_setting_value(
        &self,
        name: &str,
        volatility: CandleVolatility,
    ) -> anyhow::Result<PointSettingValue>;
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
    pub fn new(path_to_file: &str) -> anyhow::Result<Self> {
        let mut settings = Self {
            names_values: Default::default(),
        };

        let mut reader = Reader::from_path(path_to_file)
            .context("an error occurred on creating a reader from the path")?;

        for record in reader.deserialize::<CsvRecord>() {
            let record = record.context("an error on deserializing a record")?;
            settings
                .names_values
                .insert(record.setting_name, record.setting_value);
        }

        Ok(settings)
    }
}

impl Settings for ExcelFileSettings {
    fn get_point_setting_value(&self, name: &str) -> anyhow::Result<PointSettingValue> {
        let value = self
            .names_values
            .get(name)
            .context(format!("a point setting with a name {} is not found", name))?;

        value.parse::<PointSettingValue>().context(format!(
            "an error on parsing a point setting with a name {}",
            name
        ))
    }

    fn get_ratio_setting_value(
        &self,
        name: &str,
        volatility: CandleVolatility,
    ) -> anyhow::Result<PointSettingValue> {
        let ratio_value = self
            .names_values
            .get(name)
            .context(format!("a ratio setting with a name {} is not found", name))?;

        get_point_value_from_ratio(ratio_value, volatility).context(format!(
            "an error occurred during a conversion from a ratio value {} to a point value",
            name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_successfully_return_a_point_value_from_a_ratio() {
        assert_eq!(get_point_value_from_ratio("1.234k", 10.0).unwrap(), 12.34);
    }

    #[test]
    fn should_return_an_error_on_getting_a_point_value_from_invalid_ratio() {
        assert!(get_point_value_from_ratio("hello", 10.0).is_err());
    }
}
