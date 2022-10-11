use anyhow::Context;
use anyhow::Result;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::path::Path;

use csv::Reader;
use serde::{Deserialize, Serialize};

use crate::entities::candle::CandleVolatility;
use crate::entities::SIGNIFICANT_DECIMAL_PLACES;

pub type ParamName = String;
pub type ParamInputValue = String;
pub type ParamOutputValue = Decimal;

pub trait StrategyParams {
    type PointParam: Display;
    type RatioParam: Display;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamOutputValue;

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        volatility: CandleVolatility,
    ) -> ParamOutputValue;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StrategyParam {
    pub name: ParamName,
    pub value: ParamInputValue,
}

pub struct StrategyMultiSourcingParams<PointParam, RatioParam>
where
    PointParam: Display,
    RatioParam: Display,
{
    point_param_values: HashMap<ParamName, ParamOutputValue>,
    ratio_param_values: HashMap<ParamName, ParamOutputValue>,
    point_param_name: PhantomData<PointParam>,
    ratio_param_name: PhantomData<RatioParam>,
}

impl<PointSetting, RatioSetting> StrategyMultiSourcingParams<PointSetting, RatioSetting>
where
    PointSetting: Display,
    RatioSetting: Display,
{
    pub fn from_csv<P: AsRef<Path>>(path_to_file: P) -> Result<Self> {
        let mut params = Self {
            point_param_values: Default::default(),
            ratio_param_values: Default::default(),
            point_param_name: PhantomData,
            ratio_param_name: PhantomData,
        };

        let mut reader = Reader::from_path(path_to_file)
            .context("an error occurred on creating a reader from the path")?;

        for param in reader.deserialize() {
            let param: StrategyParam = param.context("an error on deserializing a setting")?;

            params.add_param(param)?;
        }

        Ok(params)
    }

    pub fn from_vec(params: Vec<StrategyParam>) -> Result<Self> {
        let mut result_params = Self {
            point_param_values: Default::default(),
            ratio_param_values: Default::default(),
            point_param_name: PhantomData,
            ratio_param_name: PhantomData,
        };

        for param in params {
            result_params.add_param(param)?;
        }

        Ok(result_params)
    }

    fn add_param(&mut self, param: StrategyParam) -> Result<()> {
        if param
            .value
            .chars()
            .last()
            .context("empty string setting value was got")?
            .is_alphabetic()
        {
            let numeric_param_value = param.value[..param.value.len() - 1]
                .parse::<ParamOutputValue>()?
                .round_dp(SIGNIFICANT_DECIMAL_PLACES);
            self.ratio_param_values
                .insert(param.name, numeric_param_value);
        } else {
            let numeric_param_value = param
                .value
                .parse::<ParamOutputValue>()?
                .round_dp(SIGNIFICANT_DECIMAL_PLACES);
            self.point_param_values
                .insert(param.name, numeric_param_value);
        }

        Ok(())
    }
}

impl<PointParam, RatioParam> StrategyParams for StrategyMultiSourcingParams<PointParam, RatioParam>
where
    PointParam: Display,
    RatioParam: Display,
{
    type PointParam = PointParam;
    type RatioParam = RatioParam;

    fn get_point_param_value(&self, name: Self::PointParam) -> ParamOutputValue {
        *self
            .point_param_values
            .get(&name.to_string())
            .unwrap_or_else(|| panic!("a point param with a name {} is not found", name))
    }

    fn get_ratio_param_value(
        &self,
        name: Self::RatioParam,
        volatility: CandleVolatility,
    ) -> ParamOutputValue {
        let ratio_value = self
            .ratio_param_values
            .get(&name.to_string())
            .unwrap_or_else(|| panic!("a ratio param with a name {} is not found", name));

        (ratio_value * Decimal::from(volatility)).round_dp(SIGNIFICANT_DECIMAL_PLACES)
    }
}

impl<PointParam, RatioParam> Display for StrategyMultiSourcingParams<PointParam, RatioParam>
where
    PointParam: Display,
    RatioParam: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Point params:")?;

        for (name, value) in self.point_param_values.iter() {
            writeln!(f, "{}: {}", name, value)?;
        }

        writeln!(f, "\nRatio params:")?;

        for (name, value) in self.ratio_param_values.iter() {
            writeln!(f, "{}: {}", name, value)?;
        }

        Ok(())
    }
}
