use std::time::SystemTime;
use chrono::NaiveDateTime;
use chrono_tz::Europe::Berlin;
use chrono::TimeZone;
use bdays::HolidayCalendar;

// Subtracts secs0 from secs1
pub fn days_diff(secs0: i64, secs1: i64) -> i32 {
    let cal = bdays::calendars::WeekendsOnly;
    let d0 = NaiveDateTime::from_timestamp_opt(secs0, 0).expect("Invalid timestamp");
    let d1 = NaiveDateTime::from_timestamp_opt(secs1, 0).expect("Invalid timestamp");
    cal.bdays(Berlin.from_utc_datetime(&d0), Berlin.from_utc_datetime(&d1))
}

pub fn secs_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use crate::time::days_diff;

    #[test]
    fn days_diff_test() {
        assert_eq!(days_diff(1671404400, 1671490800), 1);
    }

    #[test]
    fn same_day() {
        assert_eq!(days_diff(1671404100, 1671404400), 0);
        assert_eq!(days_diff(1671405000, 1671404400), 0);
    }
}
