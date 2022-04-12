use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use log::error;

const TIME_FORMAT: &str = "%F %T%.3f";

pub enum LogLevel {
    Error,
}

pub fn log_message(message: &str, log_level: LogLevel, logger_target: Option<&str>) {
    if let Some(target) = logger_target {
        match log_level {
            LogLevel::Error => {
                error!(target: target, "{}", message);
            }
        }
    } else {
        match log_level {
            LogLevel::Error => {
                error!("{}", message);
            }
        }
    }
}

pub fn to_time(time_str: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(time_str, TIME_FORMAT)
        .context(format!("Error on parsing NaiveDateTime from {}", time_str))
}
