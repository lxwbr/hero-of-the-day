use lambda::handler_fn;
use serde_json::{ Value, json };
use std::env;
use response::ok;
use model::hero::{Hero, from_dynamo_item};

extern crate rusoto_core;
extern crate rusoto_dynamodb;

use rusoto_core::{Region};
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, ScanInput};

mod error;
use error::HeroListError;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(_event: Value) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());

    let scan_input = ScanInput {
        table_name: env::var("HERO_TABLE")?,
        ..Default::default()
    };

    let heroes: Vec<Hero> = client.scan(scan_input).await?.items
        .ok_or(HeroListError::NoneScan)?
        .into_iter()
        .map(from_dynamo_item)
        .collect();

    Ok(ok(json!(heroes).to_string()))
}