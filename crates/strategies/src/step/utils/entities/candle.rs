use crate::step::utils::backtesting_charts::ChartIndex;
use base::entities::candle::{BasicCandleProperties, CandlePrice};

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct StepBacktestingCandleProperties {
    pub step_common: StepCandleProperties,
    pub chart_index: ChartIndex,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct StepCandleProperties {
    pub base: BasicCandleProperties,
    pub leading_price: CandlePrice,
}

impl AsRef<StepCandleProperties> for StepCandleProperties {
    fn as_ref(&self) -> &StepCandleProperties {
        self
    }
}

impl From<StepBacktestingCandleProperties> for StepCandleProperties {
    fn from(properties: StepBacktestingCandleProperties) -> Self {
        properties.step_common
    }
}

impl AsRef<StepCandleProperties> for StepBacktestingCandleProperties {
    fn as_ref(&self) -> &StepCandleProperties {
        &self.step_common
    }
}

impl From<StepBacktestingCandleProperties> for BasicCandleProperties {
    fn from(properties: StepBacktestingCandleProperties) -> Self {
        properties.step_common.base
    }
}

impl AsRef<BasicCandleProperties> for StepBacktestingCandleProperties {
    fn as_ref(&self) -> &BasicCandleProperties {
        &self.step_common.base
    }
}
