use std::collections::HashMap;

use aws_sdk_dynamodb::model::AttributeValue;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub email: String,
    pub last_login: Option<u64>,
}

impl User {
    pub fn from_dynamo_item(item: &HashMap<String, AttributeValue>) -> User {
        User {
            email: item["email"]
                .as_s()
                .expect("email attribute is missing in the user entry")
                .to_owned(),
            last_login: item["last_login"]
                .as_n()
                .map(|timestamp| {
                    u64::from_str(timestamp)
                        .expect("last_login attribute was not an N field")
                        .to_owned()
                })
                .ok()
                .to_owned(),
        }
    }
}
