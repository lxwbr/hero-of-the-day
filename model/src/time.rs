use std::time::SystemTime;
use chrono::NaiveDateTime;
use bdays::HolidayCalendar;

// Subtracts secs0 from secs1
pub fn days_diff(secs0: i64, secs1: i64) -> i32 {
    let cal = bdays::calendars::WeekendsOnly;
    let d0 = NaiveDateTime::from_timestamp(secs0, 0);
    let d1  = NaiveDateTime::from_timestamp(secs1, 0);
    cal.bdays(d0, d1)
}

pub fn secs_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn days_diff_test() {
    assert_eq!(days_diff(1681434005, 1681761605), 1);
}
