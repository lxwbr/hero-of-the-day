use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum HeroListError {
    NoneScan,
}

impl Error for HeroListError {}

impl Display for HeroListError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            HeroListError::NoneScan => write!(f, "Scan of league table failed!"),
        }
    }
}
