use std::fmt::{Display, Formatter};

#[derive(Copy, Clone)]
pub enum HistoricalTimeframe {
    Hour,
    ThirtyMin,
    FifteenMin,
    OneMin,
}

impl Display for HistoricalTimeframe {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            HistoricalTimeframe::Hour => write!(f, "1h"),
            HistoricalTimeframe::ThirtyMin => write!(f, "30m"),
            HistoricalTimeframe::FifteenMin => write!(f, "15m"),
            HistoricalTimeframe::OneMin => write!(f, "1m"),
        }
    }
}
