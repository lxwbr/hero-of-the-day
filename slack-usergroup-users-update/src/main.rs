use futures::{prelude::*};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use model::schedule::Schedule;
use repository::schedule::ScheduleRepository;
use repository::hero::HeroRepository;
use slack;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Request {}

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

    run(service_fn(move |_: LambdaEvent<Request>| async move {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let hero_names = hero_repository_ref.list().await?;

        let repeating_schedules: Vec<Schedule> = future::try_join_all(hero_names.iter()
        .map(|hero| {
            schedule_repository_ref.get_all_repeating_before(hero.name.clone(), secs)
        })).await?.into_iter().flatten().collect();

        println!("Repeating schedules: {:#?}", repeating_schedules);

        let schedules: Vec<Schedule> = future::try_join_all(hero_names.iter()
        .map(|hero| {
            schedule_repository_ref.get_first_before(hero.name.clone(), secs)
        })).await?.into_iter().flatten().collect();

        slack::Client::new(slack::get_slack_token().await?)
            .usergroups_users_update_with_schedules(schedules)
            .await?;

        Ok::<(), Error>(())
    })).await?;
    Ok(())
}
