use crate::step::utils::entities::angle::{BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use backtesting::Balance;
use base::entities::candle::CandlePrice;
use base::entities::{Level, Tendency};
use rust_decimal::Decimal;

pub type ChartIndex = usize;

pub enum ChartTraceEntity {
    LeadingPrice(CandlePrice),
    Tendency(Tendency),
    Balance(Balance),

    WorkingLevel {
        last_broken_angle:
            FullAngleProperties<BasicAngleProperties, StepBacktestingCandleProperties>,
    },
    StopLoss {
        working_level_chart_index: ChartIndex,
        stop_loss_price: CandlePrice,
    },
    TakeProfit {
        working_level_chart_index: ChartIndex,
        take_profit_price: CandlePrice,
    },
}

pub type AxisValue = Decimal;

pub type AmountOfCandles = usize;

#[derive(Debug)]
pub struct StepBacktestingChartTraces {
    total_amount_of_candles: AmountOfCandles,

    price: Vec<Option<AxisValue>>,
    tendency: Vec<Option<AxisValue>>,
    balance: Vec<Option<AxisValue>>,

    working_levels: Vec<Vec<Option<AxisValue>>>,
    stop_losses: Vec<Vec<Option<AxisValue>>>,
    take_profits: Vec<Vec<Option<AxisValue>>>,
}

impl StepBacktestingChartTraces {
    pub fn new(total_amount_of_candles: AmountOfCandles) -> Self {
        let price = vec![None; total_amount_of_candles];
        let tendency = vec![None; total_amount_of_candles];
        let balance = vec![None; total_amount_of_candles];

        Self {
            total_amount_of_candles,
            price,
            tendency,
            balance,
            working_levels: vec![],
            stop_losses: vec![],
            take_profits: vec![],
        }
    }

    pub fn get_total_amount_of_candles(&self) -> AmountOfCandles {
        self.total_amount_of_candles
    }

    pub fn get_price_trace_mut(&mut self) -> &mut [Option<AxisValue>] {
        &mut self.price
    }

    pub fn get_price_trace(&self) -> &[Option<AxisValue>] {
        &self.price
    }

    pub fn get_tendency_trace_mut(&mut self) -> &mut [Option<AxisValue>] {
        &mut self.tendency
    }

    pub fn get_tendency_trace(&self) -> &[Option<AxisValue>] {
        &self.tendency
    }

    pub fn get_balance_trace_mut(&mut self) -> &mut [Option<AxisValue>] {
        &mut self.balance
    }

    pub fn get_balance_trace(&self) -> &[Option<AxisValue>] {
        &self.balance
    }

    pub fn create_new_working_level_trace(&mut self) -> &mut [Option<AxisValue>] {
        self.working_levels
            .push(vec![None; self.total_amount_of_candles]);
        self.working_levels.last_mut().unwrap()
    }

    pub fn get_working_level_traces(&self) -> &[Vec<Option<AxisValue>>] {
        &self.working_levels
    }

    pub fn create_new_stop_loss_trace(&mut self) -> &mut [Option<AxisValue>] {
        self.stop_losses
            .push(vec![None; self.total_amount_of_candles]);
        self.stop_losses.last_mut().unwrap()
    }

    pub fn get_stop_loss_traces(&self) -> &[Vec<Option<AxisValue>>] {
        &self.stop_losses
    }

    pub fn create_new_take_profit_trace(&mut self) -> &mut [Option<AxisValue>] {
        self.take_profits
            .push(vec![None; self.total_amount_of_candles]);
        self.take_profits.last_mut().unwrap()
    }

    pub fn get_take_profit_traces(&self) -> &[Vec<Option<AxisValue>>] {
        &self.take_profits
    }
}

pub trait AddEntityToChartTraces {
    /// Saves additional data of the current backtesting launch for the future analysis.
    fn add_entity_to_chart_traces(
        &self,
        entity: ChartTraceEntity,
        chart_traces: &mut StepBacktestingChartTraces,
        current_candle: &StepBacktestingCandleProperties,
    );
}

#[derive(Default)]
pub struct BacktestingChartTracesModifier;

impl BacktestingChartTracesModifier {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AddEntityToChartTraces for BacktestingChartTracesModifier {
    fn add_entity_to_chart_traces(
        &self,
        entity: ChartTraceEntity,
        chart_traces: &mut StepBacktestingChartTraces,
        current_candle: &StepBacktestingCandleProperties,
    ) {
        // the current tick time position is always the next candle index
        let current_tick_candle_index =
        // if the current candle index is last, use the current candle index as the last draw point
        if current_candle.chart_index < chart_traces.get_total_amount_of_candles() - 1 {
            current_candle.chart_index + 1
        } else {
            current_candle.chart_index
        };

        match entity {
            ChartTraceEntity::LeadingPrice(current_price) => {
                chart_traces.get_price_trace_mut()[current_tick_candle_index] = Some(current_price);
            }
            ChartTraceEntity::Tendency(current_tendency) => {
                chart_traces.get_tendency_trace_mut()[current_tick_candle_index] =
                    Some(AxisValue::from(current_tendency as i32));
            }
            ChartTraceEntity::Balance(current_balance) => {
                chart_traces.get_balance_trace_mut()[current_tick_candle_index] =
                    Some(current_balance);
            }
            ChartTraceEntity::WorkingLevel { last_broken_angle } => {
                let price = if last_broken_angle.base.r#type == Level::Max {
                    last_broken_angle.candle.props.base.prices.high
                } else {
                    last_broken_angle.candle.props.base.prices.low
                };

                let working_level_trace = chart_traces.create_new_working_level_trace();

                for item in working_level_trace
                    .iter_mut()
                    .take(current_tick_candle_index + 1)
                    .skip(last_broken_angle.candle.props.chart_index)
                {
                    *item = Some(price);
                }
            }
            ChartTraceEntity::StopLoss {
                working_level_chart_index,
                stop_loss_price,
            } => {
                let stop_loss_trace = chart_traces.create_new_stop_loss_trace();

                for item in stop_loss_trace
                    .iter_mut()
                    .take(current_tick_candle_index + 1)
                    .skip(working_level_chart_index)
                {
                    *item = Some(stop_loss_price);
                }
            }
            ChartTraceEntity::TakeProfit {
                working_level_chart_index: working_level_index_chart_index,
                take_profit_price,
            } => {
                let take_profit_trace = chart_traces.create_new_take_profit_trace();

                for item in take_profit_trace
                    .iter_mut()
                    .take(current_tick_candle_index + 1)
                    .skip(working_level_index_chart_index)
                {
                    *item = Some(take_profit_price);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base::entities::Item;
    use rust_decimal_macros::dec;

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__leading_price__should_successfully_add_price_to_corresponding_array(
    ) {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let current_candle = StepBacktestingCandleProperties {
            chart_index: 2,
            base: Default::default(),
        };

        let leading_price = dec!(1.38473);

        let chart_traces_modifier = BacktestingChartTracesModifier::new();

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::LeadingPrice(leading_price),
            &mut chart_traces,
            &current_candle,
        );

        assert_eq!(
            chart_traces.get_price_trace(),
            &[None, None, None, Some(leading_price), None]
        );

        let new_current_candle = StepBacktestingCandleProperties {
            chart_index: 4,
            base: Default::default(),
        };

        let new_leading_price = dec!(1.38473);

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::LeadingPrice(new_leading_price),
            &mut chart_traces,
            &new_current_candle,
        );

        assert_eq!(
            chart_traces.get_price_trace(),
            &[
                None,
                None,
                None,
                Some(leading_price),
                Some(new_leading_price)
            ]
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__tendency__should_successfully_add_tendency_to_corresponding_array(
    ) {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let current_candle = StepBacktestingCandleProperties {
            chart_index: 2,
            base: Default::default(),
        };

        let tendency = Tendency::Up;

        let chart_traces_modifier = BacktestingChartTracesModifier::new();

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::Tendency(tendency),
            &mut chart_traces,
            &current_candle,
        );

        assert_eq!(
            chart_traces.get_tendency_trace(),
            &[
                None,
                None,
                None,
                Some(AxisValue::from(tendency as i32)),
                None
            ]
        );

        let new_current_candle = StepBacktestingCandleProperties {
            chart_index: 4,
            base: Default::default(),
        };

        let new_tendency = Tendency::Down;

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::Tendency(new_tendency),
            &mut chart_traces,
            &new_current_candle,
        );

        assert_eq!(
            chart_traces.get_tendency_trace(),
            &[
                None,
                None,
                None,
                Some(AxisValue::from(tendency as i32)),
                Some(AxisValue::from(new_tendency as i32))
            ]
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__balance__should_successfully_add_balance_to_corresponding_array()
    {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let current_candle = StepBacktestingCandleProperties {
            chart_index: 2,
            base: Default::default(),
        };

        let balance = dec!(10_000);

        let chart_traces_modifier = BacktestingChartTracesModifier::new();

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::Balance(balance),
            &mut chart_traces,
            &current_candle,
        );

        assert_eq!(
            chart_traces.get_balance_trace(),
            &[None, None, None, Some(balance), None]
        );

        let new_current_candle = StepBacktestingCandleProperties {
            chart_index: 4,
            base: Default::default(),
        };

        let new_balance = dec!(20_000);

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::Balance(new_balance),
            &mut chart_traces,
            &new_current_candle,
        );

        assert_eq!(
            chart_traces.get_balance_trace(),
            &[None, None, None, Some(balance), Some(new_balance)]
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__working_level__should_successfully_add_working_level_line_to_corresponding_array(
    ) {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let last_broken_angle = FullAngleProperties {
            base: Default::default(),
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties {
                    base: Default::default(),
                    chart_index: 1,
                },
            },
        };

        let current_candle = StepBacktestingCandleProperties {
            chart_index: 3,
            base: Default::default(),
        };

        let chart_traces_modifier = BacktestingChartTracesModifier::new();

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::WorkingLevel { last_broken_angle },
            &mut chart_traces,
            &current_candle,
        );

        let expected_working_level_price = dec!(1.30939);

        assert_eq!(
            chart_traces.get_working_level_traces()[0],
            &[
                None,
                Some(expected_working_level_price),
                Some(expected_working_level_price),
                Some(expected_working_level_price),
                Some(expected_working_level_price),
            ]
        );

        let new_last_broken_angle = FullAngleProperties {
            base: BasicAngleProperties { r#type: Level::Max },
            candle: Item {
                id: String::from("2"),
                props: StepBacktestingCandleProperties {
                    base: Default::default(),
                    chart_index: 2,
                },
            },
        };

        let new_current_candle = StepBacktestingCandleProperties {
            chart_index: 4,
            base: Default::default(),
        };

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::WorkingLevel {
                last_broken_angle: new_last_broken_angle,
            },
            &mut chart_traces,
            &new_current_candle,
        );

        let new_expected_working_level_price = dec!(1.31078);

        assert_eq!(
            chart_traces.get_working_level_traces()[1],
            &[
                None,
                None,
                Some(new_expected_working_level_price),
                Some(new_expected_working_level_price),
                Some(new_expected_working_level_price),
            ]
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__stop_loss__should_successfully_add_stop_loss_line_to_corresponding_array(
    ) {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let current_candle = StepBacktestingCandleProperties {
            chart_index: 3,
            base: Default::default(),
        };

        let stop_loss_price = dec!(1.30939);

        let chart_traces_modifier = BacktestingChartTracesModifier::new();

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::StopLoss {
                working_level_chart_index: 1,
                stop_loss_price,
            },
            &mut chart_traces,
            &current_candle,
        );

        assert_eq!(
            chart_traces.get_stop_loss_traces()[0],
            &[
                None,
                Some(stop_loss_price),
                Some(stop_loss_price),
                Some(stop_loss_price),
                Some(stop_loss_price),
            ]
        );

        let new_current_candle = StepBacktestingCandleProperties {
            chart_index: 4,
            base: Default::default(),
        };

        let new_stop_loss_price = dec!(1.40279);

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::StopLoss {
                working_level_chart_index: 2,
                stop_loss_price: new_stop_loss_price,
            },
            &mut chart_traces,
            &new_current_candle,
        );

        assert_eq!(
            chart_traces.get_stop_loss_traces()[1],
            &[
                None,
                None,
                Some(new_stop_loss_price),
                Some(new_stop_loss_price),
                Some(new_stop_loss_price),
            ]
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__take_profit__should_successfully_add_take_profit_line_to_corresponding_array(
    ) {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let current_candle = StepBacktestingCandleProperties {
            chart_index: 3,
            base: Default::default(),
        };

        let take_profit_price = dec!(1.30939);

        let chart_traces_modifier = BacktestingChartTracesModifier::new();

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::TakeProfit {
                working_level_chart_index: 1,
                take_profit_price,
            },
            &mut chart_traces,
            &current_candle,
        );

        assert_eq!(
            chart_traces.get_take_profit_traces()[0],
            &[
                None,
                Some(take_profit_price),
                Some(take_profit_price),
                Some(take_profit_price),
                Some(take_profit_price),
            ]
        );

        let new_current_candle = StepBacktestingCandleProperties {
            chart_index: 4,
            base: Default::default(),
        };

        let new_take_profit_price = dec!(1.40279);

        chart_traces_modifier.add_entity_to_chart_traces(
            ChartTraceEntity::TakeProfit {
                working_level_chart_index: 2,
                take_profit_price: new_take_profit_price,
            },
            &mut chart_traces,
            &new_current_candle,
        );

        assert_eq!(
            chart_traces.get_take_profit_traces()[1],
            &[
                None,
                None,
                Some(new_take_profit_price),
                Some(new_take_profit_price),
                Some(new_take_profit_price),
            ]
        );
    }
}
