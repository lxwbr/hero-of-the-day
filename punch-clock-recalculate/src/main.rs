use lambda_http::{run, service_fn, Error, Request, RequestExt};
use repository::schedule::ScheduleRepository;
use response::{bad_request, ok};
use serde::Deserialize;
use model::punch_clock::recalculate_punch_time;
use repository::punch_clock::PunchClockRepository;

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
    let punch_clock_repository_ref = &PunchClockRepository::new(&shared_config);

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("hero") {
            Some(hero) => {
                let schedules = schedule_repository_ref.get(hero.to_string().clone(), None).await?;
                let recalculated = recalculate_punch_time(hero.to_string().clone(), schedules);
                for punch_clock in recalculated.into_iter() {
                    punch_clock_repository_ref.put(&punch_clock).await?;
                };
                ok(())
            }
            None => bad_request("Could not parse JSON payload for schedule update".into())
        }
    })).await?;
    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct Payload {}
