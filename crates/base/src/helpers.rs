use chrono::{Datelike, Duration, NaiveDateTime, Weekday};
use rust_decimal::Decimal;

use crate::entities::LOT;

pub type PointValue = Decimal;
pub type PriceValue = Decimal;

pub fn points_to_price(points: PointValue) -> PriceValue {
    points / Decimal::from(LOT)
}

pub fn price_to_points(price: PriceValue) -> PointValue {
    price * Decimal::from(LOT)
}

pub fn mean(numbers: &[Decimal]) -> Decimal {
    let sum: Decimal = numbers.iter().sum();
    sum / Decimal::from(numbers.len())
}

type Day = u32;
type Month = u32;

pub struct Holiday {
    pub day: Day,
    pub month: Month,
}

impl PartialEq<NaiveDateTime> for Holiday {
    fn eq(&self, other: &NaiveDateTime) -> bool {
        other.day() == self.day && other.month() == self.month
    }
}

impl PartialEq<Holiday> for NaiveDateTime {
    fn eq(&self, other: &Holiday) -> bool {
        self.day() == other.day && self.month() == other.month
    }
}

pub type NumberOfDaysToExclude = u32;

pub fn exclude_weekend_and_holidays(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    holidays: &[Holiday],
) -> NumberOfDaysToExclude {
    let mut days_to_exclude = 0;
    let mut current_date = start_time;

    while current_date < end_time {
        match current_date.weekday() {
            Weekday::Sat | Weekday::Sun => {
                days_to_exclude += 1;
            }
            _ => {
                for holiday in holidays {
                    if holiday == &current_date {
                        days_to_exclude += 1;
                    }
                }
            }
        }

        current_date += Duration::days(1);
    }

    days_to_exclude
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    #[allow(non_snake_case)]
    fn exclude_weekend_and_holidays__contains_all_weekend_and_one_holiday__should_return_3() {
        let start = NaiveDate::from_ymd(2020, 12, 24).and_hms(0, 0, 0);
        let end = NaiveDate::from_ymd(2020, 12, 29).and_hms(0, 0, 0);

        let holidays = vec![Holiday { day: 25, month: 12 }];

        assert_eq!(exclude_weekend_and_holidays(start, end, &holidays), 3);
    }

    #[test]
    #[allow(non_snake_case)]
    fn exclude_weekend_and_holidays__contains_no_weekend_and_holidays__should_return_0() {
        let start = NaiveDate::from_ymd(2022, 8, 8).and_hms(0, 0, 0);
        let end = NaiveDate::from_ymd(2022, 8, 12).and_hms(0, 0, 0);

        let holidays = vec![Holiday { day: 25, month: 12 }];

        assert_eq!(exclude_weekend_and_holidays(start, end, &holidays), 0);
    }
}
