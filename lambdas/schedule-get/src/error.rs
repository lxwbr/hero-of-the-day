use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum ScheduleGetError {
    HeroParameterMissing,
}

impl Error for ScheduleGetError {}

impl Display for ScheduleGetError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ScheduleGetError::HeroParameterMissing => {
                write!(f, "`hero` is missing in `pathParameters`!")
            }
        }
    }
}
