use anyhow::Context;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fmt::Display;
use std::marker::PhantomData;
use std::path::Path;

use csv::Reader;
use serde::{Deserialize, Serialize};

use crate::entities::candle::CandleVolatility;
use crate::entities::SIGNIFICANT_DECIMAL_PLACES;

pub type ParamName = String;
pub type CsvParamValue = String;
pub type ParamValue = Decimal;

pub trait StrategyParams {
    type PointParam: Display;
    type RatioParam: Display;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamValue;

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        volatility: CandleVolatility,
    ) -> ParamValue;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StrategyCsvParam {
    pub name: ParamName,
    pub value: CsvParamValue,
}

pub struct StrategyCsvFileParams<PointParam, RatioParam>
where
    PointParam: Display,
    RatioParam: Display,
{
    point_param_values: HashMap<ParamName, ParamValue>,
    ratio_param_values: HashMap<ParamName, ParamValue>,
    point_param_name: PhantomData<PointParam>,
    ratio_param_name: PhantomData<RatioParam>,
}

impl<PointSetting, RatioSetting> StrategyCsvFileParams<PointSetting, RatioSetting>
where
    PointSetting: Display,
    RatioSetting: Display,
{
    pub fn new<P: AsRef<Path>>(path_to_file: P) -> anyhow::Result<Self> {
        let mut params = Self {
            point_param_values: Default::default(),
            ratio_param_values: Default::default(),
            point_param_name: PhantomData,
            ratio_param_name: PhantomData,
        };

        let mut reader = Reader::from_path(path_to_file)
            .context("an error occurred on creating a reader from the path")?;

        for param in reader.deserialize() {
            let param: StrategyCsvParam = param.context("an error on deserializing a setting")?;

            if param
                .value
                .chars()
                .last()
                .context("empty string setting value was got")?
                .is_alphabetic()
            {
                let numeric_setting_value = param.value[..param.value.len() - 1]
                    .parse::<ParamValue>()?
                    .round_dp(SIGNIFICANT_DECIMAL_PLACES);
                params
                    .ratio_param_values
                    .insert(param.name, numeric_setting_value);
            } else {
                let numeric_setting_value = param
                    .value
                    .parse::<ParamValue>()?
                    .round_dp(SIGNIFICANT_DECIMAL_PLACES);
                params
                    .point_param_values
                    .insert(param.name, numeric_setting_value);
            }
        }

        Ok(params)
    }
}

impl<PointParam, RatioParam> StrategyParams for StrategyCsvFileParams<PointParam, RatioParam>
where
    PointParam: Display,
    RatioParam: Display,
{
    type PointParam = PointParam;
    type RatioParam = RatioParam;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamValue {
        *self
            .point_param_values
            .get(&name.to_string())
            .unwrap_or_else(|| panic!("a point param with a name {} is not found", name))
    }

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        volatility: CandleVolatility,
    ) -> ParamValue {
        let ratio_value = self
            .ratio_param_values
            .get(&name.to_string())
            .unwrap_or_else(|| panic!("a ratio param with a name {} is not found", name));

        (ratio_value * Decimal::from(volatility)).round_dp(SIGNIFICANT_DECIMAL_PLACES)
    }
}
