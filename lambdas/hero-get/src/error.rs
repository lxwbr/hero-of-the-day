use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum HeroGetError {
    HeroParameterMissing,
}

impl Error for HeroGetError {}

impl Display for HeroGetError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            HeroGetError::HeroParameterMissing => {
                write!(f, "`hero` is missing in `pathParameters`!")
            }
        }
    }
}
