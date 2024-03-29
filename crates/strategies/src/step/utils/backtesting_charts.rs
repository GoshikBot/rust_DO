use crate::step::utils::entities::angle::{BasicAngleProperties, FullAngleProperties};
use crate::step::utils::entities::candle::StepBacktestingCandleProperties;
use backtesting::Balance;
use base::entities::candle::CandlePrice;
use base::entities::tick::TickPrice;
use base::entities::{Level, Tendency};
use rust_decimal::Decimal;

pub type ChartIndex = usize;

#[derive(Debug, Eq, PartialEq)]
pub enum ChartTraceEntity<'a> {
    Tendency(Tendency),
    Balance(Balance),

    WorkingLevel {
        crossed_angle:
            &'a FullAngleProperties<BasicAngleProperties, StepBacktestingCandleProperties>,
    },
    StopLoss {
        working_level_chart_index: ChartIndex,
        stop_loss_price: CandlePrice,
    },
    TakeProfit {
        working_level_chart_index: ChartIndex,
        take_profit_price: CandlePrice,
    },
    ClosePrice {
        working_level_chart_index: ChartIndex,
        close_price: TickPrice,
    },
}

pub type AxisValue = Decimal;

pub type AmountOfCandles = usize;

#[derive(Debug)]
pub struct StepBacktestingChartTraces {
    total_amount_of_candles: AmountOfCandles,

    tendency: Vec<Option<AxisValue>>,
    balance: Vec<Option<AxisValue>>,

    working_levels: Vec<Vec<Option<AxisValue>>>,
    stop_losses: Vec<Vec<Option<AxisValue>>>,
    take_profits: Vec<Vec<Option<AxisValue>>>,
    close_prices: Vec<Vec<Option<AxisValue>>>,
}

impl StepBacktestingChartTraces {
    pub fn new(total_amount_of_candles: AmountOfCandles) -> Self {
        let tendency = vec![None; total_amount_of_candles];
        let balance = vec![None; total_amount_of_candles];

        Self {
            total_amount_of_candles,
            tendency,
            balance,
            working_levels: vec![],
            stop_losses: vec![],
            take_profits: vec![],
            close_prices: vec![],
        }
    }

    pub fn get_total_amount_of_candles(&self) -> AmountOfCandles {
        self.total_amount_of_candles
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

    pub fn create_new_close_price_trace(&mut self) -> &mut [Option<AxisValue>] {
        self.close_prices
            .push(vec![None; self.total_amount_of_candles]);
        self.close_prices.last_mut().unwrap()
    }

    pub fn get_close_price_traces(&self) -> &[Vec<Option<AxisValue>>] {
        &self.close_prices
    }
}

#[derive(Default)]
pub struct BacktestingChartTracesModifier;

impl BacktestingChartTracesModifier {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn add_entity_to_chart_traces(
    entity: ChartTraceEntity,
    chart_traces: &mut StepBacktestingChartTraces,
    current_candle_chart_index: ChartIndex,
) {
    let total_amount_of_candles = chart_traces.get_total_amount_of_candles();

    // the current tick time position is always the next candle index
    let current_tick_candle_index =
        // if the current candle index is last, use the current candle index as the last draw point
        if current_candle_chart_index < total_amount_of_candles - 1 {
            current_candle_chart_index + 1
        } else {
            current_candle_chart_index
        };

    match entity {
        ChartTraceEntity::Tendency(current_tendency) => {
            chart_traces.get_tendency_trace_mut()[current_candle_chart_index] =
                Some(AxisValue::from(current_tendency as i32));
        }
        ChartTraceEntity::Balance(current_balance) => {
            chart_traces.get_balance_trace_mut()[current_candle_chart_index] =
                Some(current_balance);
        }
        ChartTraceEntity::WorkingLevel {
            crossed_angle: last_broken_angle,
        } => {
            let price = if last_broken_angle.base.r#type == Level::Max {
                last_broken_angle.candle.props.step_common.base.prices.high
            } else {
                last_broken_angle.candle.props.step_common.base.prices.low
            };

            let working_level_trace = chart_traces.create_new_working_level_trace();

            for item in working_level_trace
                .iter_mut()
                .take(current_candle_chart_index + 1)
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
            working_level_chart_index,
            take_profit_price,
        } => {
            let take_profit_trace = chart_traces.create_new_take_profit_trace();

            for item in take_profit_trace
                .iter_mut()
                .take(current_tick_candle_index + 1)
                .skip(working_level_chart_index)
            {
                *item = Some(take_profit_price);
            }
        }
        ChartTraceEntity::ClosePrice {
            working_level_chart_index,
            close_price,
        } => {
            let close_price_trace = chart_traces.create_new_close_price_trace();

            for item in close_price_trace
                .iter_mut()
                .take(current_tick_candle_index + 1)
                .skip(working_level_chart_index)
            {
                *item = Some(close_price);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::utils::entities::angle::AngleState;
    use crate::step::utils::entities::candle::StepCandleProperties;
    use base::entities::candle::BasicCandleProperties;
    use base::entities::{CandlePrices, Item};
    use rust_decimal_macros::dec;

    #[test]
    #[allow(non_snake_case)]
    fn add_entity_to_chart_traces__tendency__should_successfully_add_tendency_to_corresponding_array(
    ) {
        let mut chart_traces = StepBacktestingChartTraces::new(5);

        let current_candle_chart_index = 3;

        let tendency = Tendency::Up;

        add_entity_to_chart_traces(
            ChartTraceEntity::Tendency(tendency),
            &mut chart_traces,
            current_candle_chart_index,
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

        let new_current_candle_chart_index = 4;

        let new_tendency = Tendency::Down;

        add_entity_to_chart_traces(
            ChartTraceEntity::Tendency(new_tendency),
            &mut chart_traces,
            new_current_candle_chart_index,
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

        let current_candle_chart_index = 3;

        let balance = dec!(10_000);

        add_entity_to_chart_traces(
            ChartTraceEntity::Balance(balance),
            &mut chart_traces,
            current_candle_chart_index,
        );

        assert_eq!(
            chart_traces.get_balance_trace(),
            &[None, None, None, Some(balance), None]
        );

        let new_current_candle_chart_index = 4;

        let new_balance = dec!(20_000);

        add_entity_to_chart_traces(
            ChartTraceEntity::Balance(new_balance),
            &mut chart_traces,
            new_current_candle_chart_index,
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

        let crossed_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Min,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("1"),
                props: StepBacktestingCandleProperties {
                    chart_index: 1,
                    step_common: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.28000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                },
            },
        };

        let current_candle_chart_index = 3;

        add_entity_to_chart_traces(
            ChartTraceEntity::WorkingLevel {
                crossed_angle: &crossed_angle,
            },
            &mut chart_traces,
            current_candle_chart_index,
        );

        let expected_working_level_price = dec!(1.28000);

        assert_eq!(
            chart_traces.get_working_level_traces()[0],
            &[
                None,
                Some(expected_working_level_price),
                Some(expected_working_level_price),
                Some(expected_working_level_price),
                None
            ]
        );

        let new_crossed_angle = FullAngleProperties {
            base: BasicAngleProperties {
                r#type: Level::Max,
                state: AngleState::Real,
            },
            candle: Item {
                id: String::from("2"),
                props: StepBacktestingCandleProperties {
                    chart_index: 2,
                    step_common: StepCandleProperties {
                        base: BasicCandleProperties {
                            prices: CandlePrices {
                                low: dec!(1.30000),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                },
            },
        };

        let new_current_candle_chart_index = 4;

        add_entity_to_chart_traces(
            ChartTraceEntity::WorkingLevel {
                crossed_angle: &new_crossed_angle,
            },
            &mut chart_traces,
            new_current_candle_chart_index,
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

        let current_candle_chart_index = 3;

        let stop_loss_price = dec!(1.30939);

        add_entity_to_chart_traces(
            ChartTraceEntity::StopLoss {
                working_level_chart_index: 1,
                stop_loss_price,
            },
            &mut chart_traces,
            current_candle_chart_index,
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

        let new_current_candle_chart_index = 4;

        let new_stop_loss_price = dec!(1.40279);

        add_entity_to_chart_traces(
            ChartTraceEntity::StopLoss {
                working_level_chart_index: 2,
                stop_loss_price: new_stop_loss_price,
            },
            &mut chart_traces,
            new_current_candle_chart_index,
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

        let current_candle_chart_index = 3;

        let take_profit_price = dec!(1.30939);

        add_entity_to_chart_traces(
            ChartTraceEntity::TakeProfit {
                working_level_chart_index: 1,
                take_profit_price,
            },
            &mut chart_traces,
            current_candle_chart_index,
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

        let new_current_candle_chart_index = 4;

        let new_take_profit_price = dec!(1.40279);

        add_entity_to_chart_traces(
            ChartTraceEntity::TakeProfit {
                working_level_chart_index: 2,
                take_profit_price: new_take_profit_price,
            },
            &mut chart_traces,
            new_current_candle_chart_index,
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
