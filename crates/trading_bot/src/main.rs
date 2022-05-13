use polars::datatypes::Float32Chunked;
use polars::prelude::{Float64Chunked, RollingOptions, Series};
use std::collections::{LinkedList, VecDeque};

fn main() {
    // let vector = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    // let s: Series = vector.iter().collect();
    // let res = s
    //     .rolling_mean(RollingOptions {
    //         window_size: 3,
    //         min_periods: 3,
    //         weights: None,
    //         center: false,
    //     })
    //     .unwrap();
    //
    // let res = res.f32().unwrap();
    //
    // let res = res
    //     .into_iter()
    //     .filter(|v| v.is_some())
    //     .collect::<Float32Chunked>();
    // println!("{:?}", res);

    let v1 = 14.9795267482957;
    let v2 = v1 as f32;
    println!("{}, {}", v1, v2);
}
