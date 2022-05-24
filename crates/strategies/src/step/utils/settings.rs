use anyhow::Context;
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::path::Path;

use csv::Reader;
use serde::{Deserialize, Serialize};

use base::entities::candle::CandleVolatility;

pub type SettingName = String;
pub type CsvSettingValue = String;
pub type PointSettingValue = f32;

pub trait Settings {
    type PointSetting: Display;
    type RatioSetting: Display;

    fn get_point_setting_value(&self, name: Self::PointSetting) -> PointSettingValue;

    fn get_ratio_setting_value(
        &self,
        name: Self::RatioSetting,
        volatility: CandleVolatility,
    ) -> PointSettingValue;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CsvSetting {
    pub name: SettingName,
    pub value: CsvSettingValue,
}

pub struct CsvFileSettings<PointSetting, RatioSetting>
where
    PointSetting: Display,
    RatioSetting: Display,
{
    point_setting_values: HashMap<SettingName, PointSettingValue>,
    ratio_setting_values: HashMap<SettingName, PointSettingValue>,
    point_setting_name: PhantomData<PointSetting>,
    ratio_setting_name: PhantomData<RatioSetting>,
}

impl<PointSetting, RatioSetting> CsvFileSettings<PointSetting, RatioSetting>
where
    PointSetting: Display,
    RatioSetting: Display,
{
    pub fn new<P: AsRef<Path>>(path_to_file: P) -> anyhow::Result<Self> {
        let mut settings = Self {
            point_setting_values: Default::default(),
            ratio_setting_values: Default::default(),
            point_setting_name: PhantomData,
            ratio_setting_name: PhantomData,
        };

        let mut reader = Reader::from_path(path_to_file)
            .context("an error occurred on creating a reader from the path")?;

        for setting in reader.deserialize() {
            let setting: CsvSetting = setting.context("an error on deserializing a setting")?;

            if setting
                .value
                .chars()
                .last()
                .context("empty string setting value was got")?
                .is_alphabetic()
            {
                let numeric_setting_value =
                    (&setting.value[..setting.value.len() - 1]).parse::<PointSettingValue>()?;
                settings
                    .ratio_setting_values
                    .insert(setting.name, numeric_setting_value);
            } else {
                let numeric_setting_value = setting.value.parse::<PointSettingValue>()?;
                settings
                    .point_setting_values
                    .insert(setting.name, numeric_setting_value);
            }
        }

        Ok(settings)
    }
}

impl<PointSetting, RatioSetting> Settings for CsvFileSettings<PointSetting, RatioSetting>
where
    PointSetting: Display,
    RatioSetting: Display,
{
    type PointSetting = PointSetting;
    type RatioSetting = RatioSetting;

    fn get_point_setting_value(&self, name: Self::PointSetting) -> PointSettingValue {
        *self
            .point_setting_values
            .get(&name.to_string())
            .unwrap_or_else(|| panic!("a point setting with a name {} is not found", name))
    }

    fn get_ratio_setting_value(
        &self,
        name: Self::RatioSetting,
        volatility: CandleVolatility,
    ) -> PointSettingValue {
        let ratio_value = self
            .ratio_setting_values
            .get(&name.to_string())
            .unwrap_or_else(|| panic!("a ratio setting with a name {} is not found", name));

        ratio_value * volatility
    }
}
