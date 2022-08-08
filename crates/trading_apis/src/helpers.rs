use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};

const TIME_FORMAT: &str = "%F %T%.3f";

pub fn from_naive_str_to_naive_datetime(time_str: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(time_str, TIME_FORMAT)
        .context(format!("error on parsing NaiveDateTime from {}", time_str))
}

pub fn from_iso_utc_str_to_utc_datetime(time_str: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::from(
        DateTime::parse_from_rfc3339(time_str)
            .context(format!("error on parsing UTC datetime from {}", time_str))?,
    ))
}
