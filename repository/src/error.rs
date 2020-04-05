use std::fmt::{ Display, Result, Formatter };
use std::error::Error;

#[derive(Debug)]
pub enum RepositoryError {
    NoneScan
}

impl Error for RepositoryError {}

impl Display for RepositoryError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            RepositoryError::NoneScan => write!(f, "Scan of league table failed!")
        }
    }
}
