use rusoto_dynamodb::AttributeValue;
use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;

extern crate chrono;
use chrono::prelude::*;

#[derive(Debug)]
pub struct Schedule {
    pub hero: String,
    pub shift_start_time: i64,
    pub assignees: Vec<String>,
}

impl Schedule {
    pub fn from_dynamo_item(item: HashMap<String, AttributeValue>) -> Schedule {
        Schedule {
            hero: item["hero"]
                .s
                .as_ref()
                .expect("hero attribute is missing in the League entry")
                .to_owned(),
            shift_start_time: i64::from_str(
                item["shift_start_time"]
                    .n
                    .as_ref()
                    .expect("shift_start_time attribute is missing in the League entry"),
            )
            .expect("shift_start_time attribute was not an N field")
            .to_owned(),
            assignees: match item.get("assignees") {
                Some(assignees) => assignees.ss.as_ref().unwrap_or(&Vec::new()).to_owned(),
                None => Vec::new(),
            },
        }
    }
}

impl Serialize for Schedule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let naive = NaiveDateTime::from_timestamp(self.shift_start_time, 0);
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

        let mut s = serializer.serialize_struct("Schedule", 2)?;
        s.serialize_field("hero", &self.hero)?;
        s.serialize_field("shift_start_time", &datetime.to_rfc3339())?;
        s.serialize_field("assignees", &self.assignees)?;
        s.end()
    }
}
