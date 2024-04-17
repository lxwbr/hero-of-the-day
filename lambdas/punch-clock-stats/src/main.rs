use lambda_http::{run, service_fn, Error, Request, RequestExt};
use model::punch_clock::PunchClock;
use model::schedule::Schedule;
use model::time::secs_now;
use repository::punch_clock::PunchClockRepository;
use repository::schedule::ScheduleRepository;
use response::{bad_request, ok};
use serde::Serialize;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let shared_config = aws_config::load_from_env().await;
    let punch_clock_repository_ref = &PunchClockRepository::new(&shared_config);
    let schedule_repository_ref = &ScheduleRepository::new(&shared_config);

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("hero") {
            Some(hero) => {
                let hero_string = hero.to_string();
                let punch_cards: Vec<PunchClock> =
                    punch_clock_repository_ref.get_all(hero_string).await?;

                match schedule_repository_ref
                    .get_first_before(hero.to_string(), secs_now())
                    .await?
                {
                    None => ok(Response {
                        punch_cards,
                        current_schedule: None,
                    }),
                    Some(schedule) => ok(Response {
                        punch_cards,
                        current_schedule: Some(schedule),
                    }),
                }
            }
            None => bad_request("Could not parse JSON payload for schedule update".into()),
        }
    }))
    .await?;
    Ok(())
}

#[derive(Serialize)]
struct Response {
    punch_cards: Vec<PunchClock>,
    current_schedule: Option<Schedule>,
}
