#![type_length_limit="1126348"]

mod error;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
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
use slack;
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
    .unwrap()
    .with_timezone(&Utc);

    let user_timezone = body["timezone"]
        .as_str()
        .expect("Expected `timezone`, e.g. Europe/Berlin");

    let today_start = midnight(user_timezone);

    println!("shift_start_time: {}", shift_start_time);
    println!("today_start: {}", today_start);

    let duration = shift_start_time
        .signed_duration_since(today_start)
        .num_seconds();

    if duration < 0 {
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

        let schedule_repository = ScheduleRepository::new(&client);
        let schedule_option = schedule_repository
            .update_assignees(
                &operation,
                hero.clone(),
                shift_start_time.timestamp(),
                assignees.clone(),
            )
            .await?;

        println!("Updated the schedule: {:?}", schedule_option);

        // If it is an ADD operation, update the hero table to include the e-mail address to the members list.
        if let Operation::Add = operation {
            let hero_repository = HeroRepository::new(&client);
            hero_repository.append_members(hero.to_string(), assignees).await?;
        }

        if duration == 0 {
            // Need to load the rest of the users for that day
            match ScheduleRepository::new(&client).get_first_before(hero, shift_start_time.timestamp() as u64).await? {
                Some(schedule) => slack::Client::new(slack::get_slack_token().await?)
                    .usergroups_users_update_with_schedules(vec!(schedule))
                    .await?,
                None => ()
            }
        }

        Ok(ok(json!(()).to_string()))
    }
}

fn midnight(timezone: &str) -> DateTime<Tz> {
    let tz: Tz = timezone.parse().unwrap();
    Utc::now()
        .with_timezone(&tz)
        .date()
        .and_hms(0, 0, 0)
}
