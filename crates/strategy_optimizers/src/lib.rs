use pyo3::prelude::*;
use rand::Rng;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn test_function(p: f32) -> PyResult<f32> {
    Ok(rand::thread_rng().gen_range(0..51) as f32 * p)
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn strategy_optimizers(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(test_function, m)?)?;
    Ok(())
}
