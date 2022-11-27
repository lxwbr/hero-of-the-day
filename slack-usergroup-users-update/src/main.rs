use futures::{prelude::*, stream::futures_unordered::FuturesUnordered};
use lambda_http::{run, service_fn, Error};
use repository::schedule::ScheduleRepository;
use repository::hero::HeroRepository;
use response::{ok};
use slack;
use std::time::SystemTime;

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

    run(service_fn(move |_| async move {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let hero_names = hero_repository_ref.list().await?;

        let mut schedule_futures = hero_names
            .iter()
            .map(|hero| {
                schedule_repository_ref.get_first_before(hero.name.clone(), secs)
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

        ok(())
    })).await?;
    Ok(())
}
