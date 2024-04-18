use std::collections::HashMap;

use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub email: String,
    pub last_login: Option<u64>,
    pub last_seen_release_notes: Option<String>,
}

impl From<&HashMap<String, AttributeValue>> for User {
    fn from(item: &HashMap<String, AttributeValue>) -> Self {
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
            last_seen_release_notes: item.get("last_seen_release_notes").map(|value| {
                value
                    .as_s()
                    .expect("last_seen_release_notes should be a string")
                    .to_owned()
            }),
        }
    }
}
