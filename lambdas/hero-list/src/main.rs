mod error;

use error::HeroListError;
use lambda_runtime::{handler_fn, Context};
use model::hero::Hero;
use response::ok;
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, ScanInput};
use serde_json::{json, Value};
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(_event: Value, _: Context) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());

    let scan_input = ScanInput {
        table_name: env::var("HERO_TABLE")?,
        ..Default::default()
    };

    let heroes: Vec<Hero> = client
        .scan(scan_input)
        .await?
        .items
        .ok_or(HeroListError::NoneScan)?
        .into_iter()
        .map(Hero::from_dynamo_item)
        .collect();

    Ok(ok(json!(heroes).to_string()))
}
