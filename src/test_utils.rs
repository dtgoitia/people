use std::str::FromStr;

use chrono::NaiveDate;

pub fn d(s: &str) -> NaiveDate {
    NaiveDate::from_str(s).expect(&format!("Invalid date: {s}"))
}
