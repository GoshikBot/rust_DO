use crate::entities::LOT;

pub fn points_to_price(points: f32) -> f32 {
    points / LOT as f32
}

pub fn price_to_points(price: f32) -> f32 {
    price * LOT as f32
}

pub fn mean(numbers: &[f32]) -> f32 {
    let sum: f32 = numbers.iter().sum();
    sum / numbers.len() as f32
}
