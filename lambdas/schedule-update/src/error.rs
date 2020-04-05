use std::fmt::{ Display, Result, Formatter };
use std::error::Error;

#[derive(Debug)]
pub enum ScheduleUpdateError {
    HeroParameterMissing,
    AssigneesMissing
}

impl Error for ScheduleUpdateError {}

impl Display for ScheduleUpdateError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ScheduleUpdateError::HeroParameterMissing => write!(f, "`hero` is missing in `pathParameters`!"),
            ScheduleUpdateError::AssigneesMissing => write!(f, "`assignees` not specified in the body")
        }
    }
}
