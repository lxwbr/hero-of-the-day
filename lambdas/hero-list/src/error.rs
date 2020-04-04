use std::fmt::{ Display, Result, Formatter };
use std::error::Error;

#[derive(Debug)]
pub enum HeroListError {
    NoneScan
}

impl Error for HeroListError {}

impl Display for HeroListError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            HeroListError::NoneScan => write!(f, "Scan of league table failed!")
        }
    }
}
