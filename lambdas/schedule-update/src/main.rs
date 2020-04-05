use lambda::handler_fn;
use serde_json::{ Value, json, from_str };
use response::ok;
extern crate chrono;
use chrono::{DateTime};

extern crate rusoto_core;
extern crate rusoto_dynamodb;

use rusoto_core::{Region};
use rusoto_dynamodb::{DynamoDbClient};

mod error;
use error::ScheduleUpdateError;
use repository::{ schedule::ScheduleRepository, hero::HeroRepository };

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(event: Value) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());

    let schedule_repository = ScheduleRepository::new(&client);

    let hero =  event["pathParameters"]["hero"].as_str().ok_or(ScheduleUpdateError::HeroParameterMissing)?.to_string();

    let body: Value = from_str(event["body"].as_str().unwrap())?;
    let shift_start_time = DateTime::parse_from_rfc3339(
        body["shift_start_time"].as_str().expect("shift_start_time has to be a rfc3339 string")
    ).unwrap().timestamp();
    let assignees = vec!(body["assignees"].as_str().unwrap().to_string());

    let schedule = schedule_repository.append_assignee(hero.clone(), shift_start_time, assignees.clone()).await?;

    println!("Updated the schedule: {:?}", schedule);

    let hero_repository = HeroRepository::new(&client);

    hero_repository.append_members(hero, assignees).await?;

    Ok(ok(json!(schedule).to_string()))
}
