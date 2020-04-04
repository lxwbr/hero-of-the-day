use serde::{ Serialize, Deserialize };
use std::collections::HashMap;
use rusoto_dynamodb::AttributeValue;

#[derive(Serialize, Deserialize, Debug)]
pub struct Hero {
    pub name: String,
    pub members: Vec<String>
}

impl Hero {
    pub fn from_dynamo_item(item: HashMap<String, AttributeValue>) -> Hero {
        Hero {
            name: item["name"].s.as_ref().expect("name attribute is missing in the League entry").to_owned(),
            members: item["members"].ss.as_ref().unwrap_or(&Vec::new()).to_owned()
        }
    }
}

