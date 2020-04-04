use lambda::handler_fn;
use serde_json::{ Value, json };
use std::env;
use response::ok;
use model::schedule::Schedule;

extern crate rusoto_core;
extern crate rusoto_dynamodb;

use rusoto_core::{Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, AttributeValue, QueryInput};
use maplit::hashmap;

mod error;
use error::ScheduleGetError;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(event: Value) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());

    let hero =  event["pathParameters"]["hero"].as_str().ok_or(ScheduleGetError::HeroParameterMissing)?;
    let attribute_values = hashmap!{
        ":hero".to_owned() => AttributeValue {
            s: Some(hero.to_string()),
            ..Default::default()
        }
    };

    let query_input = QueryInput {
        table_name: env::var("SCHEDULE_TABLE")?,
        key_condition_expression: Some("hero = :hero".to_string()),
        expression_attribute_values: Some(attribute_values),
        ..Default::default()
    };

    let schedules: Vec<Schedule> = client.query(query_input).await?.items
        .ok_or(ScheduleGetError::NoneScan)?
        .into_iter()
        .map(Schedule::from_dynamo_item)
        .collect();
    Ok(ok(json!(schedules).to_string()))
}
