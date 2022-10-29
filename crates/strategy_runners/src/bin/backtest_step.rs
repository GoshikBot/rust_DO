use anyhow::Result;
use backtesting::historical_data::get_historical_data;
use backtesting::historical_data::serialization::HistoricalDataCsvSerialization;
use backtesting::historical_data::synchronization::sync_candles_and_ticks;
use backtesting::trading_engine::BacktestingTradingEngine;
use backtesting::{get_path_name_for_data_config, HistoricalData, StrategyInitConfig};
use base::corridor::BasicCorridorUtilsImpl;
use base::entities::candle::BasicCandleProperties;
use base::entities::{StrategyTimeframes, Timeframe, CANDLE_TIMEFRAME_ENV, TICK_TIMEFRAME_ENV};
use base::helpers::exclude_weekend_and_holidays;
use base::requests::ureq::UreqRequestApi;
use chrono::{DateTime, Duration};
use plotly::common::{Marker, Title};
use plotly::layout::{Axis, GridPattern, LayoutGrid};
use plotly::{Candlestick, Layout, Plot, Scatter};
use rust_decimal_macros::dec;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;
use trading_apis::metaapi_market_data_api::{
    ApiData, ApiUrls, AUTH_TOKEN_ENV, DEMO_ACCOUNT_ID_ENV, MAIN_API_URL_ENV,
    MARKET_DATA_API_URL_ENV,
};
use trading_apis::MetaapiMarketDataApi;

use base::params::StrategyMultiSourcingParams;
use strategies::step::step_backtesting::run_iteration;
use strategies::step::utils::angle_utils::AngleUtilsImpl;
use strategies::step::utils::backtesting_charts::{
    add_entity_to_chart_traces, AxisValue, StepBacktestingChartTraces,
};
use strategies::step::utils::corridors::CorridorsImpl;
use strategies::step::utils::entities::candle::StepCandleProperties;
use strategies::step::utils::entities::params::{StepPointParam, StepRatioParam};
use strategies::step::utils::entities::{
    Mode, MODE_ENV, STEP_HISTORICAL_DATA_FOLDER_ENV, STEP_PARAMS_CSV_FILE_ENV,
};
use strategies::step::utils::helpers::HelpersImpl;
use strategies::step::utils::level_conditions::LevelConditionsImpl;
use strategies::step::utils::level_utils::LevelUtilsImpl;
use strategies::step::utils::order_utils::OrderUtilsImpl;
use strategies::step::utils::stores::in_memory_step_backtesting_store::InMemoryStepBacktestingStore;
use strategies::step::utils::stores::{StepBacktestingConfig, StepBacktestingStores};
use strategies::step::utils::trading_limiter::TradingLimiterBacktesting;
use strategies::step::utils::{get_candle_leading_price, StepBacktestingUtils};
use strategy_runners::step::backtesting_runner;
use strategy_runners::step::backtesting_runner::StepStrategyRunningConfig;

const PLOT_FOLDER_ENV: &str = "PLOT_FOLDER";

fn main() -> Result<()> {
    dotenv::from_filename("common.env").unwrap();
    dotenv::from_filename("step.env").unwrap();

    let candle_timeframe = "1h";
    env::set_var(CANDLE_TIMEFRAME_ENV, candle_timeframe);
    let candle_timeframe = Timeframe::from_str(candle_timeframe).unwrap();

    let tick_timeframe = "5m";
    env::set_var(TICK_TIMEFRAME_ENV, tick_timeframe);
    let tick_timeframe = Timeframe::from_str(tick_timeframe).unwrap();

    env::set_var(MODE_ENV, "debug");

    backtest_step_strategy(StrategyInitConfig {
        symbol: String::from("GBPUSDm"),
        timeframes: StrategyTimeframes {
            candle: candle_timeframe,
            tick: tick_timeframe,
        },
        end_time: DateTime::from(
            DateTime::parse_from_str("27-09-2022 18:00 +0000", "%d-%m-%Y %H:%M %z").unwrap(),
        ),
        duration: Duration::weeks(11),
    })
}

fn backtest_step_strategy(strategy_config: StrategyInitConfig) -> Result<()> {
    if (strategy_config.timeframes.candle as u32) < (strategy_config.timeframes.tick as u32) {
        anyhow::bail!("candle timeframe should be bigger than tick timeframe");
    }

    let api_data = ApiData {
        auth_token: dotenv::var(AUTH_TOKEN_ENV).unwrap(),
        account_id: dotenv::var(DEMO_ACCOUNT_ID_ENV).unwrap(),
        urls: ApiUrls {
            main: dotenv::var(MAIN_API_URL_ENV).unwrap(),
            market_data: dotenv::var(MARKET_DATA_API_URL_ENV).unwrap(),
        },
    };

    let request_api = UreqRequestApi::new();

    let market_data_api = MetaapiMarketDataApi::new(api_data, Default::default(), request_api);

    let step_historical_data_folder = dotenv::var(STEP_HISTORICAL_DATA_FOLDER_ENV).unwrap();

    let historical_data_csv_serialization = HistoricalDataCsvSerialization::new();

    let historical_data = get_historical_data(
        step_historical_data_folder,
        &strategy_config,
        &market_data_api,
        &historical_data_csv_serialization,
        sync_candles_and_ticks,
    )?;

    let historical_data = HistoricalData {
        candles: historical_data
            .candles
            .into_iter()
            .map(|candle| {
                candle.map(|c| {
                    let leading_price = get_candle_leading_price(&c);

                    StepCandleProperties {
                        base: c,
                        leading_price,
                    }
                })
            })
            .collect(),
        ticks: historical_data.ticks,
    };

    let mut step_stores = StepBacktestingStores {
        main: InMemoryStepBacktestingStore::new(),
        config: StepBacktestingConfig::default(historical_data.candles.len()),
        statistics: Default::default(),
    };

    let step_params_csv_file = dotenv::var(STEP_PARAMS_CSV_FILE_ENV).unwrap();
    let step_params: StrategyMultiSourcingParams<StepPointParam, StepRatioParam> =
        StrategyMultiSourcingParams::from_csv(step_params_csv_file)?;

    let trading_limiter = TradingLimiterBacktesting::new();

    let utils: StepBacktestingUtils<
        HelpersImpl,
        LevelUtilsImpl,
        LevelConditionsImpl,
        OrderUtilsImpl,
        BasicCorridorUtilsImpl,
        CorridorsImpl,
        AngleUtilsImpl,
        _,
        _,
        _,
    > = StepBacktestingUtils::new(
        add_entity_to_chart_traces,
        exclude_weekend_and_holidays,
        BacktestingTradingEngine::new(),
    );

    let strategy_performance = backtesting_runner::loop_through_historical_data(
        &historical_data,
        StepStrategyRunningConfig {
            timeframes: strategy_config.timeframes,
            stores: &mut step_stores,
            utils: &utils,
            params: &step_params,
        },
        &trading_limiter,
        &run_iteration,
    )?;

    println!("Strategy performance: {}", strategy_performance);
    println!(
        "Initial balance: {}",
        step_stores.config.trading_engine.balances.initial
    );
    println!(
        "Final balance: {}",
        step_stores.config.trading_engine.balances.real
    );
    println!("{:#?}", step_stores.statistics);

    if Mode::from_str(&dotenv::var(MODE_ENV).unwrap()).unwrap() != Mode::Optimization {
        let plot_file_name = get_path_name_for_data_config(&strategy_config);
        plot_results(
            historical_data.candles,
            step_stores.config.chart_traces,
            plot_file_name,
        );
    }

    Ok(())
}

fn merge_traces(traces: &[Vec<Option<AxisValue>>]) -> Vec<Vec<Option<AxisValue>>> {
    let mut merged_traces = Vec::new();
    let mut merged_traces_indices = HashSet::new();

    loop {
        let mut current_merged_trace = Vec::new();
        for (trace_index, trace) in traces.iter().enumerate() {
            if !merged_traces_indices.contains(&trace_index) {
                // if it's the first trace to merge, just add it to the current merged trace
                if current_merged_trace.is_empty() {
                    current_merged_trace = trace.clone();
                    merged_traces_indices.insert(trace_index);
                } else {
                    let mut merge_trace = true;

                    // check if the candidate trace lies within the unfilled area of the current merged trace
                    for (x_index, candidate_y_value) in trace.iter().enumerate() {
                        if let Some(candidate_y_value) = candidate_y_value {
                            if let Some(current_merged_trace_y_value) =
                                &current_merged_trace[x_index]
                            {
                                // if two points have the same y value, they can be merged
                                if candidate_y_value != current_merged_trace_y_value {
                                    merge_trace = false;
                                    break;
                                }
                            }
                        }
                    }

                    if merge_trace {
                        // merge the candidate trace
                        for (x_index, candidate_y_value) in trace.iter().enumerate() {
                            if let Some(candidate_y_value) = candidate_y_value {
                                current_merged_trace[x_index] = Some(*candidate_y_value);
                            }
                        }

                        // mark the trace as merged
                        merged_traces_indices.insert(trace_index);
                    }
                }
            }
        }

        merged_traces.push(current_merged_trace);

        // if all traces are merged, break the loop
        if merged_traces_indices.len() == traces.len() {
            break;
        }
    }

    merged_traces
}

fn plot_results(
    candles: Vec<Option<StepCandleProperties>>,
    chart_traces: StepBacktestingChartTraces,
    file_name: impl AsRef<Path>,
) {
    let x = candles
        .iter()
        .map(|candle| {
            candle
                .clone()
                .map(|c| c.base.time.format("%Y-%m-%d %H:%M:%S").to_string())
        })
        .collect::<Vec<_>>();

    let leading_price = Scatter::new(
        x.clone(),
        candles
            .iter()
            .map(|candle| candle.as_ref().map(|c| c.leading_price))
            .collect::<Vec<_>>(),
    )
    .y_axis("y1")
    .name("leading price")
    .text_array((0..candles.len()).map(|i| i.to_string()).collect());

    let tendency = Scatter::new(x.clone(), chart_traces.get_tendency_trace().to_vec())
        .y_axis("y2")
        .name("tendency")
        .text_array(
            (0..chart_traces.get_tendency_trace().len())
                .map(|i| i.to_string())
                .collect(),
        );

    let balance = Scatter::new(x.clone(), chart_traces.get_balance_trace().to_vec())
        .y_axis("y3")
        .name("balance")
        .text_array(
            (0..chart_traces.get_balance_trace().len())
                .map(|i| i.to_string())
                .collect(),
        );

    let open = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.base.prices.open))
        .collect::<Vec<_>>();
    let high = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.base.prices.high))
        .collect::<Vec<_>>();
    let low = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.base.prices.low))
        .collect::<Vec<_>>();
    let close = candles
        .iter()
        .map(|candle| candle.clone().map(|c| c.base.prices.close))
        .collect::<Vec<_>>();

    let candle = Box::new(
        Candlestick::new(x.clone(), open, high, low, close)
            .y_axis("y4")
            .name("candle")
            .text_array((0..candles.len()).map(|i| i.to_string()).collect()),
    );

    let mut working_level_traces = Vec::new();
    for (i, trace) in merge_traces(chart_traces.get_working_level_traces())
        .iter()
        .enumerate()
    {
        working_level_traces.push(
            Scatter::new(
                x.clone(),
                trace
                    .iter()
                    .map(|t| t.map(|v| v + dec!(0.00005)))
                    .collect::<Vec<_>>(),
            )
            .y_axis("y4")
            .name(format!("working level {}", i))
            .text_array((0..candles.len()).map(|i| i.to_string()).collect())
            .marker(Marker::new().color("yellow")),
        );
    }

    let mut take_profit_traces = Vec::new();
    for (i, trace) in merge_traces(chart_traces.get_take_profit_traces())
        .iter()
        .enumerate()
    {
        take_profit_traces.push(
            Scatter::new(x.clone(), trace.to_vec())
                .y_axis("y4")
                .name(format!("take profit {}", i))
                .text_array((0..candles.len()).map(|i| i.to_string()).collect())
                .marker(Marker::new().color("green")),
        );
    }

    let mut stop_loss_traces = Vec::new();
    for (i, trace) in merge_traces(chart_traces.get_stop_loss_traces())
        .iter()
        .enumerate()
    {
        stop_loss_traces.push(
            Scatter::new(x.clone(), trace.to_vec())
                .y_axis("y4")
                .name(format!("stop loss {}", i))
                .text_array((0..candles.len()).map(|i| i.to_string()).collect())
                .marker(Marker::new().color("red")),
        );
    }

    let mut close_price_traces = Vec::new();
    for (i, trace) in merge_traces(chart_traces.get_close_price_traces())
        .iter()
        .enumerate()
    {
        close_price_traces.push(
            Scatter::new(x.clone(), trace.to_vec())
                .y_axis("y4")
                .name(format!("close price  {}", i))
                .text_array((0..candles.len()).map(|i| i.to_string()).collect())
                .marker(Marker::new().color("blue")),
        );
    }

    let layout = Layout::new()
        .title(Title::new(file_name.as_ref().to_str().unwrap()))
        .y_axis(Axis::new().domain(&[0.61, 1.]).fixed_range(false))
        .y_axis2(Axis::new().domain(&[0.51, 0.6]))
        .y_axis3(Axis::new().domain(&[0.41, 0.5]))
        .y_axis4(Axis::new().domain(&[0., 0.4]).fixed_range(false))
        .grid(
            LayoutGrid::new()
                .rows(4)
                .columns(1)
                .pattern(GridPattern::Independent),
        )
        .height(3045);

    let mut plot = Plot::new();

    plot.add_trace(leading_price);
    plot.add_trace(tendency);
    plot.add_trace(balance);
    plot.add_trace(candle);

    for trace in working_level_traces {
        plot.add_trace(trace);
    }

    for trace in take_profit_traces {
        plot.add_trace(trace);
    }

    for trace in stop_loss_traces {
        plot.add_trace(trace);
    }

    for trace in close_price_traces {
        plot.add_trace(trace);
    }

    plot.set_layout(layout);

    let mut plot_path = PathBuf::new();
    plot_path.push(dotenv::var(PLOT_FOLDER_ENV).unwrap());
    plot_path.push(file_name);
    plot_path.set_extension("html");

    plot.write_html(plot_path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn merge_traces__should_properly_merge_traces() {
        let traces = vec![
            vec![
                Some(dec!(1)),
                Some(dec!(1)),
                Some(dec!(1)),
                None,
                None,
                None,
                None,
            ],
            vec![
                Some(dec!(1)),
                Some(dec!(1)),
                Some(dec!(1)),
                None,
                None,
                None,
                None,
            ],
            vec![
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                None,
            ],
            vec![
                None,
                Some(dec!(3)),
                Some(dec!(3)),
                Some(dec!(3)),
                None,
                None,
                None,
            ],
            vec![
                None,
                None,
                None,
                Some(dec!(2)),
                Some(dec!(2)),
                Some(dec!(2)),
                Some(dec!(2)),
            ],
            vec![
                None,
                None,
                Some(dec!(4)),
                Some(dec!(4)),
                Some(dec!(4)),
                None,
                None,
            ],
            vec![
                None,
                None,
                Some(dec!(4)),
                Some(dec!(4)),
                Some(dec!(4)),
                None,
                None,
            ],
            vec![None, None, None, None, None, Some(dec!(4)), Some(dec!(4))],
        ];

        let expected_merged_traces = vec![
            vec![
                Some(dec!(1)),
                Some(dec!(1)),
                Some(dec!(1)),
                Some(dec!(2)),
                Some(dec!(2)),
                Some(dec!(2)),
                Some(dec!(2)),
            ],
            vec![
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                Some(dec!(5)),
                None,
            ],
            vec![
                None,
                Some(dec!(3)),
                Some(dec!(3)),
                Some(dec!(3)),
                None,
                Some(dec!(4)),
                Some(dec!(4)),
            ],
            vec![
                None,
                None,
                Some(dec!(4)),
                Some(dec!(4)),
                Some(dec!(4)),
                None,
                None,
            ],
        ];

        assert_eq!(expected_merged_traces, merge_traces(&traces));
    }
}
