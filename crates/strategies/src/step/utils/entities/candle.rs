use crate::step::utils::backtesting_charts::ChartIndex;
use base::entities::candle::BasicCandleProperties;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct StepBacktestingCandleProperties {
    pub base: BasicCandleProperties,
    pub chart_index: ChartIndex,
}

impl From<StepBacktestingCandleProperties> for BasicCandleProperties {
    fn from(properties: StepBacktestingCandleProperties) -> Self {
        properties.base
    }
}

impl AsRef<BasicCandleProperties> for StepBacktestingCandleProperties {
    fn as_ref(&self) -> &BasicCandleProperties {
        &self.base
    }
}
