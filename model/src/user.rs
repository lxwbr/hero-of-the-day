use serde::{ Serialize, Deserialize };
use std::collections::HashMap;
use rusoto_dynamodb::{AttributeValue};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub email: String,
    pub last_login: Option<u64>
}

impl User {
    pub fn from_dynamo_item(item: HashMap<String, AttributeValue>) -> User {
        User {
            email: item["email"].s.as_ref().expect("name attribute is missing in the League entry").to_owned(),
            last_login: item["last_login"].n.as_ref().map(|timestamp| u64::from_str(timestamp).expect("last_login attribute was not an N field").to_owned())
        }
    }
}

