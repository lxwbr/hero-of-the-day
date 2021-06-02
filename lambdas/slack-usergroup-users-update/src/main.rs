#![type_length_limit="1123558"]

use futures::{prelude::*, stream::futures_unordered::FuturesUnordered};
use lambda_runtime::{handler_fn, Context};
use repository::{hero::HeroRepository, schedule::ScheduleRepository};
use rusoto_core::Region;
use rusoto_dynamodb::DynamoDbClient;
use serde_json::Value;
use slack;
use std::time::SystemTime;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(_event: Value, _: Context) -> Result<(), Error> {
    let dynamodb_client = DynamoDbClient::new(Region::default());
    let hero_repository = HeroRepository::new(&dynamodb_client);
    let hero_names = hero_repository.list_names().await?;

    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let mut schedule_futures = hero_names
        .iter()
        .map(|hero_name| {
            ScheduleRepository::new(&dynamodb_client).get_first_before(hero_name.clone(), secs)
        })
        .collect::<FuturesUnordered<_>>();

    let mut schedules = Vec::new();
    while let Some(schedule_result) = schedule_futures.next().await {
        match schedule_result {
            Ok(schedule_option) => match schedule_option {
                Some(schedule) => schedules.push(schedule),
                None => println!("no schedule"),
            },
            Err(e) => println!("{:?}", e),
        }
    }

    slack::Client::new(slack::get_slack_token().await?)
        .usergroups_users_update_with_schedules(schedules)
        .await?;

    Ok(())
}
