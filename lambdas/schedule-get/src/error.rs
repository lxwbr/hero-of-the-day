use std::fmt::{ Display, Result, Formatter };
use std::error::Error;

#[derive(Debug)]
pub enum ScheduleGetError {
    HeroParameterMissing
}

impl Error for ScheduleGetError {}

impl Display for ScheduleGetError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ScheduleGetError::HeroParameterMissing => write!(f, "`hero` is missing in `pathParameters`!")
        }
    }
}
