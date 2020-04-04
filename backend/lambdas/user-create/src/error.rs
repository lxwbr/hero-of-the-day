use std::fmt::{ Display, Result, Formatter };
use std::error::Error;

#[derive(Debug)]
pub enum UserPutError {
    NoEmailProvided
}

impl Error for UserPutError {}

impl Display for UserPutError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            UserPutError::NoEmailProvided => write!(f, "No `email` parameter provided")
        }
    }
}
