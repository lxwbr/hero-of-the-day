use std::collections::HashMap;
use aws_sdk_dynamodb::model::{AttributeValue};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Hero {
    pub name: String,
    pub members: Vec<String>,
}

impl Hero {
    pub fn from_dynamo_item(item: &HashMap<String, AttributeValue>) -> Hero {
        Hero {
            name: item["name"]
                .as_s()
                .expect("name attribute is missing in the League entry")
                .to_owned(),
            members: item["members"]
                .as_ss()
                .unwrap_or(&Vec::new())
                .to_owned(),
        }
    }
}
