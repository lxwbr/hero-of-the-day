use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum UserPutError {
    NoEmailProvided,
}

impl Error for UserPutError {}

impl Display for UserPutError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            UserPutError::NoEmailProvided => write!(f, "No `email` parameter provided"),
        }
    }
}
