use anyhow::anyhow;
use aws_sdk_dynamodb::types::AttributeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Hero {
    pub name: String,
    pub members: Vec<String>,
    pub channel: Option<String>,
}

impl TryFrom<&HashMap<String, AttributeValue>> for Hero {
    type Error = anyhow::Error;

    fn try_from(value: &HashMap<String, AttributeValue>) -> Result<Self, Self::Error> {
        let name = value["name"]
            .as_s()
            .map_err(|err| anyhow!("name attribute is missing in the hero entry: {:?}", err))?
            .to_owned();

        let members = value["members"].as_ss().unwrap_or(&Vec::new()).to_owned();

        let channel = value
            .get("channel")
            .map(|attr| attr.as_s().unwrap_or(&"".to_string()).to_owned());

        Ok(Hero {
            name,
            members,
            channel,
        })
    }
}
