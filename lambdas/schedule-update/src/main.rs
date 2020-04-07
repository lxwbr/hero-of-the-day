mod error;

use chrono::{DateTime, Local};
use error::ScheduleUpdateError;
use lambda::handler_fn;
use repository::{
    hero::HeroRepository,
    schedule::{Operation, ScheduleRepository},
};
use response::{bad_request, ok};
use rusoto_core::Region;
use rusoto_dynamodb::DynamoDbClient;
use serde_json::{from_str, json, Value};
use std::str::FromStr;

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

    let hero = event["pathParameters"]["hero"]
        .as_str()
        .ok_or(ScheduleUpdateError::HeroParameterMissing)?
        .to_string();

    let body: Value = from_str(event["body"].as_str().unwrap())?;
    let shift_start_time = DateTime::parse_from_rfc3339(
        body["shift_start_time"]
            .as_str()
            .expect("`shift_start_time` has to be a rfc3339 string"),
    )
    .unwrap();

    // TODO: make local time part of input params
    let today_start = Local::today().and_hms(0, 0, 0);

    if shift_start_time.le(&today_start) {
        let message = json!({
            "message":
                format!(
                    "Provided date is {}. You cannot change the past. Even batman can't.",
                    shift_start_time.to_rfc2822()
                )
        });
        Ok(bad_request(message.to_string()))
    } else {
        let assignees: Vec<String> = body["assignees"]
            .as_array()
            .ok_or(ScheduleUpdateError::AssigneesMissing)?
            .into_iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("Expected assignees entries to be strings")
                    .to_string()
            })
            .collect();

        let operation = Operation::from_str(
            body["operation"]
                .as_str()
                .expect("`operation` not specified in the body"),
        )
        .expect("`operation` has to be of type ADD or DELETE");

        let schedule = schedule_repository
            .update_assignees(
                &operation,
                hero.clone(),
                shift_start_time.timestamp(),
                assignees.clone(),
            )
            .await?;

        println!("Updated the schedule: {:?}", schedule);

        // If it is an ADD operation, update the hero table to include the e-mail address to the members list.
        if let Operation::Add = operation {
            let hero_repository = HeroRepository::new(&client);
            hero_repository.append_members(hero, assignees).await?;
        }

        Ok(ok(json!(schedule).to_string()))
    }
}
