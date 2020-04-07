use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum SlackUsergroupUsersUpdateError {
    NotOk,
}

impl Error for SlackUsergroupUsersUpdateError {}

impl Display for SlackUsergroupUsersUpdateError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            SlackUsergroupUsersUpdateError::NotOk => {
                write!(f, "Slack response field `ok` is false!")
            }
        }
    }
}
