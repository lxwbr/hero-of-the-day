mod slack;

use futures::{prelude::*, stream::futures_unordered::FuturesUnordered};
use lambda::handler_fn;
use repository::{hero::HeroRepository, schedule::ScheduleRepository};
use rusoto_core::Region;
use rusoto_dynamodb::DynamoDbClient;
use serde_json::Value;
use std::collections::HashMap;
use std::time::SystemTime;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(_event: Value) -> Result<(), Error> {
    let slack_token = slack::get_slack_token().await?;
    let slack_client = slack::Client::new(slack_token.clone());

    let mut usergroup_id_map = HashMap::new();
    for usergroup in slack_client.usergroups_list().await? {
        usergroup_id_map.insert(usergroup.handle, usergroup.id);
    }

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

    while let Some(schedule_result) = schedule_futures.next().await {
        match schedule_result {
            Ok(schedule_option) => match schedule_option {
                Some(schedule) => match usergroup_id_map.get(&schedule.hero) {
                    Some(usergroup_id) => {
                        let mut user_ids = Vec::new();
                        let mut user_id_results = schedule
                            .assignees
                            .iter()
                            .map(|assignee| {
                                slack::Client::new(slack_token.clone()).lookup_by_email(assignee.clone())
                            })
                            .collect::<FuturesUnordered<_>>();

                        while let Some(user_result) = user_id_results.next().await {
                            match user_result {
                                Ok(user) => user_ids.push(user.id),
                                Err(e) => println!("Got error back: {}", e),
                            }
                        }

                        slack::Client::new(slack_token.clone()).usergroups_users_update(usergroup_id.clone(), user_ids.clone()).await?;

                        println!("{}: {:?}", usergroup_id, user_ids)
                    }
                    None => println!("no usergroup id"),
                },
                None => println!("no schedule"),
            },
            Err(e) => println!("{:?}", e),
        }
    }

    Ok(())
}
