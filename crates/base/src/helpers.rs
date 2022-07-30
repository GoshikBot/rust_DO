use rust_decimal::{Decimal};

use crate::entities::LOT;

pub fn points_to_price(points: Decimal) -> Decimal {
    points / Decimal::from(LOT)
}

pub fn price_to_points(price: Decimal) -> Decimal {
    price * Decimal::from(LOT)
}

pub fn mean(numbers: &[Decimal]) -> Decimal {
    let sum: Decimal = numbers.iter().sum();
    sum / Decimal::from(numbers.len())
}
