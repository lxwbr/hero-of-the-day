use lambda::handler_fn;
use serde_json::{ Value, json };
use response::ok;

extern crate rusoto_core;
extern crate rusoto_dynamodb;

use rusoto_core::{Region};
use rusoto_dynamodb::{DynamoDbClient};

mod error;
use error::ScheduleGetError;
use repository::schedule::ScheduleRepository;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(event: Value) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());
    let repository = ScheduleRepository::new(&client);

    let hero =  event["pathParameters"]["hero"].as_str().ok_or(ScheduleGetError::HeroParameterMissing)?;

    let schedules = repository.get(hero.to_string()).await?;

    Ok(ok(json!(schedules).to_string()))
}
