use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use lambda_http::{run, service_fn, Error, Request, RequestExt};
use repository::hero::HeroRepository;
use repository::schedule::{Operation, ScheduleRepository};
use response::{bad_request, ok};
use serde::Deserialize;
use serde_json::json;
use slack;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let shared_config = aws_config::load_from_env().await;
    let schedule_repository_ref = &ScheduleRepository::new(&shared_config);
    let hero_repository_ref = &HeroRepository::new(&shared_config);

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("hero") {
            Some(hero) => {
                match event.payload::<Payload>()? {
                    Some(payload) => {
                        let shift_start_time = DateTime::parse_from_rfc3339(payload.shift_start_time.as_str())
                            .expect("`shift_start_time` has to be a rfc3339 string")
                            .with_timezone(&Utc);
                        let today_start = midnight(&payload.timezone);
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
                            bad_request(message.to_string())
                        } else {
                            if let Some(days) = payload.repeat_every_n_days {
                                println!("Request to repeat every {:?} days", days);
                            };

                            let operation = Operation::from_str(&payload.operation,)
                                .expect("`operation` has to be of type ADD or DELETE");

                            let schedule_option = schedule_repository_ref
                                .update_assignees(
                                    &operation,
                                    &hero.to_string(),
                                    shift_start_time.timestamp(),
                                    payload.assignees.clone(),
                                )
                                .await?;

                            println!("Updated the schedule: {:?}", schedule_option);

                            // If it is an ADD operation, update the hero table to include the e-mail address to the members list.
                            if let Operation::Add = operation {
                                hero_repository_ref.update_members(hero.to_string(), payload.assignees, repository::hero::UpdateOperation::Add).await?;
                            }

                            if duration == 0 {
                                // Need to load the rest of the users for that day
                                match schedule_repository_ref.get_first_before(hero.to_string(), shift_start_time.timestamp() as u64).await? {
                                    Some(schedule) => slack::Client::new(slack::get_slack_token().await?)
                                        .usergroups_users_update_with_schedules(vec!(schedule))
                                        .await?,
                                    None => ()
                                }
                            }

                            ok(())
                        }
                    },
                    None => bad_request("Could not parse JSON payload for schedule update".into())
                }
            },
            _ => bad_request("Hero parameter missing".into())
        }
    })).await?;
    Ok(())
}

fn midnight(timezone: &str) -> DateTime<Tz> {
    let tz: Tz = timezone.parse().unwrap();
    Utc::now().with_timezone(&tz).date().and_hms(0, 0, 0)
}

#[derive(Deserialize, Debug, Clone)]
struct Payload {
    timezone: String,
    shift_start_time: String,
    assignees: Vec<String>,
    repeat_every_n_days: Option<i64>,
    operation: String,
}
