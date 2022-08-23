use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub type OrderId = String;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrderType {
    Buy = 1,
    Sell = -1,
}

pub type OrderPrice = Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct BasicOrderPrices {
    pub open: OrderPrice,
    pub stop_loss: OrderPrice,
    pub take_profit: OrderPrice,
}

impl Default for BasicOrderPrices {
    fn default() -> Self {
        Self {
            open: dec!(1.38),
            stop_loss: dec!(1.37),
            take_profit: dec!(1.39),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum OrderStatus {
    Pending = 0,
    Opened = 1,
    Closed = -1,
}

impl Default for OrderStatus {
    fn default() -> Self {
        Self::Pending
    }
}

pub type OrderVolume = Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct BasicOrderProperties {
    pub r#type: OrderType,
    pub volume: OrderVolume,
    pub status: OrderStatus,
    pub prices: BasicOrderPrices,
}

impl AsRef<BasicOrderProperties> for BasicOrderProperties {
    fn as_ref(&self) -> &BasicOrderProperties {
        self
    }
}

impl Default for BasicOrderProperties {
    fn default() -> Self {
        Self {
            r#type: OrderType::Buy,
            volume: dec!(0.03),
            status: Default::default(),
            prices: Default::default(),
        }
    }
}
