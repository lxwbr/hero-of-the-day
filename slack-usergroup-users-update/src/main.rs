use futures::prelude::*;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use model::punch_clock::PunchClock;
use model::schedule::Schedule;
use repository::hero::HeroRepository;
use repository::punch_clock::PunchClockRepository;
use repository::schedule::{LastTwoSchedules, ScheduleRepository};
use serde::{Deserialize, Serialize};
use slack;
use std::time::SystemTime;
use model::time::{days_diff, secs_now};

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
    let punch_clock_repository_ref = &PunchClockRepository::new(&shared_config);

    run(service_fn(move |_: LambdaEvent<Request>| async move {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let hero_names = hero_repository_ref.list().await?;

        let repeating_schedules: Vec<Schedule> =
            future::try_join_all(hero_names.iter().map(|hero| {
                schedule_repository_ref.get_all_repeating_before(hero.name.clone(), secs)
            }))
            .await?
            .into_iter()
            .flatten()
            .collect();

        println!("Repeating schedules: {:#?}", repeating_schedules);

        let schedules_last_two: Vec<LastTwoSchedules> = future::try_join_all(
            hero_names
                .iter()
                .map(|hero| schedule_repository_ref.get_last_two_before(hero.name.clone(), secs)),
        )
        .await?
        .into_iter()
        .flatten()
        .collect();

        update_schedules_according_to_previous(punch_clock_repository_ref, &schedules_last_two)
            .await;

        let schedules: Vec<Schedule> = schedules_last_two.into_iter().map(|s| s.last).collect();

        slack::Client::new(slack::get_slack_token().await?)
            .usergroups_users_update_with_schedules(schedules)
            .await?;

        Ok::<(), Error>(())
    }))
    .await?;
    Ok(())
}

async fn update_schedules_according_to_previous(
    punch_clock_repository: &PunchClockRepository,
    last_two_schedules_vec: &Vec<LastTwoSchedules>,
) -> () {
    future::join_all(
        last_two_schedules_vec
            .into_iter()
            .map(|last_two_schedules| {
                update_according_to_previous(
                    punch_clock_repository,
                    &last_two_schedules.previous_to_last,
                )
            }),
    )
    .await;
}

async fn update_according_to_previous(
    punch_clock_repository: &PunchClockRepository,
    previous_to_last: &Option<Schedule>,
) -> () {
    match previous_to_last {
        Some(previous) => {
            let hero = &previous.hero;
            let shift_start_time = &previous.shift_start_time;
            future::join_all(previous.assignees.clone().into_iter().map(|member| {
                get_punch_clock_and_update(
                    punch_clock_repository,
                    hero.clone(),
                    member.clone(),
                    shift_start_time.clone(),
                )
            }))
            .await;
        }
        None => {} // TODO
    }
}

async fn get_punch_clock_and_update(
    punch_clock_repository: &PunchClockRepository,
    hero: String,
    member: String,
    shift_start_time: i64,
) -> Result<(), Error> {
    let days = days_diff(shift_start_time.clone(), secs_now() as i64) as u64;
    match punch_clock_repository.get(&hero, &member).await? {
        None => {
            update_punch_clock(
                punch_clock_repository,
                hero.clone(),
                member.clone(),
                days,
                shift_start_time,
                shift_start_time
            )
            .await
        }
        Some(punch_clock) => {
            if punch_clock.last_punch != shift_start_time {
                update_punch_clock(
                    punch_clock_repository,
                    hero.clone(),
                    member.clone(),
                    days + punch_clock.days,
                    punch_clock.first_punch,
                    shift_start_time
                )
                .await
            } else {
                Ok(())
            }
        }
    }
}

async fn update_punch_clock(
    punch_clock_repository: &PunchClockRepository,
    hero: String,
    member: String,
    days: u64,
    first_punch: i64,
    last_punch: i64,
) -> Result<(), Error> {
    punch_clock_repository
        .put(&PunchClock {
            hero,
            member,
            days,
            first_punch,
            last_punch,
        })
        .await
}
