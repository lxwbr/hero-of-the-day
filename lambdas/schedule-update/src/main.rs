use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use email_address::EmailAddress;
use lambda_http::{run, service_fn, Error, Request, RequestExt, RequestPayloadExt};
use repository::hero::HeroRepository;
use repository::schedule::{Operation, ScheduleRepository};
use response::{bad_request, ok};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::json;

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
                                    hero,
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
                                if let Some(schedule) = schedule_repository_ref.get_first_before(hero.to_string(), shift_start_time.timestamp() as u64).await? {
                                    let client = slack::Client::new(slack::get_slack_token().await?);
                                    client.usergroups_users_update_with_schedules(vec!(schedule.clone())).await?;
                                    let hero = hero_repository_ref.get(schedule.hero.clone()).await?;
                                    if let Some(channel) = hero.channel {
                                        client.post_message(&channel, &schedule.hero, schedule.assignees.clone()).await?
                                    }
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
    #[serde(deserialize_with = "deserialize_emails")]
    assignees: Vec<EmailAddress>,
    repeat_every_n_days: Option<i64>,
    operation: String,
}

fn deserialize_emails<'de, D>(deserializer: D) -> Result<Vec<EmailAddress>, D::Error>
where
    D: Deserializer<'de>,
{
    struct EmailAddressesVisitor;

    impl<'de> Visitor<'de> for EmailAddressesVisitor {
        type Value = Vec<EmailAddress>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an email addresses map")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut result = vec![];

            while let Some(element) = seq.next_element::<String>()? {
                match EmailAddress::from_str(&element) {
                    Ok(email) => {
                        result.push(email);
                    }
                    Err(err) => {
                        eprintln!("Failed to parse email: {}", err);
                    }
                }
            }

            Ok(result)
        }
    }

    deserializer.deserialize_seq(EmailAddressesVisitor)
}
