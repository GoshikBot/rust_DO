use crate::step::utils::backtesting_charts::ChartIndex;
use base::entities::candle::BasicCandleProperties;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct BacktestingCandleProperties {
    pub base: BasicCandleProperties,
    pub chart_index: ChartIndex,
}

impl From<BacktestingCandleProperties> for BasicCandleProperties {
    fn from(properties: BacktestingCandleProperties) -> Self {
        properties.base
    }
}
