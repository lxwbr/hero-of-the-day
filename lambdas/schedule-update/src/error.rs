use std::fmt::{ Display, Result, Formatter };
use std::error::Error;

#[derive(Debug)]
pub enum ScheduleUpdateError {
    HeroParameterMissing
}

impl Error for ScheduleUpdateError {}

impl Display for ScheduleUpdateError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ScheduleUpdateError::HeroParameterMissing => write!(f, "`hero` is missing in `pathParameters`!")
        }
    }
}
