use base::entities::order::OrderId;

use crate::{Balance, Leverage, Spread, Trades, Units};
use anyhow::Result;

pub trait BacktestingBaseStore {
    fn get_initial_balance(&self) -> Result<Balance>;

    fn get_processing_balance(&self) -> Result<Balance>;
    fn update_processing_balance(&mut self, new_processing_balance: Balance) -> Result<()>;

    fn get_real_balance(&self) -> Result<Balance>;
    fn update_real_balance(&mut self, new_real_balance: Balance) -> Result<()>;

    fn get_units(&self) -> Result<Units>;
    fn update_units(&mut self, new_units: Units) -> Result<()>;

    fn get_trades(&self) -> Result<Trades>;
    fn update_trades(&mut self, new_trades: Trades) -> Result<()>;

    fn get_leverage(&self) -> Result<Leverage>;
    fn get_use_spread(&self) -> Result<bool>;
    fn get_spread(&self) -> Result<Spread>;
}
