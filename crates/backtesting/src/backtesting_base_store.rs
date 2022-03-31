use base::entities::order::OrderId;
use simple_error::SimpleResult;

use crate::{Balance, Units, Trades, Leverage, Spread};

pub trait BacktestingBaseStore {
    fn get_initial_balance(&self) -> SimpleResult<Balance>;

    fn get_processing_balance(&self) -> SimpleResult<Balance>;
    fn update_processing_balance(&mut self, new_processing_balance: Balance) -> SimpleResult<()>;

    fn get_real_balance(&self) -> SimpleResult<Balance>;
    fn update_real_balance(&mut self, new_real_balance: Balance) -> SimpleResult<()>;

    fn get_units(&self) -> SimpleResult<Units>;
    fn update_units(&mut self, new_units: Units) -> SimpleResult<()>;

    fn get_trades(&self) -> SimpleResult<Trades>;
    fn update_trades(&mut self, new_trades: Trades) -> SimpleResult<()>;

    fn get_leverage(&self) -> SimpleResult<Leverage>;
    fn get_use_spread(&self) -> SimpleResult<bool>;
    fn get_spread(&self) -> SimpleResult<Spread>;

    fn add_limit_order(&mut self, order_id: OrderId) -> SimpleResult<()>;
    fn remove_limit_order(&mut self, order_id: OrderId) -> SimpleResult<()>;
}
