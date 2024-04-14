use lambda_http::{run, service_fn, Error, Request};
use model::hero::Hero;
use model::punch_clock::recalculate_punch_time;
use model::time::secs_now;
use repository::hero::HeroRepository;
use repository::punch_clock::PunchClockRepository;
use repository::schedule::ScheduleRepository;
use response::ok;
use serde::Deserialize;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let shared_config = aws_config::load_from_env().await;
    let hero_repository_ref = &HeroRepository::new(&shared_config);
    let schedule_repository_ref = &ScheduleRepository::new(&shared_config);
    let punch_clock_repository_ref = &PunchClockRepository::new(&shared_config);

    run(service_fn(move |_event: Request| async move {
        let heroes: Vec<Hero> = hero_repository_ref.list().await?;

        for hero in heroes.into_iter() {
            let schedules = schedule_repository_ref
                .get(hero.name.to_string().clone(), Some((0, secs_now() as i64)))
                .await?;
            let recalculated = recalculate_punch_time(hero.name.to_string().clone(), schedules);
            for punch_clock in recalculated.into_iter() {
                punch_clock_repository_ref.put(&punch_clock).await?;
            }
        }
        ok(())
    }))
        .await?;
    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct Payload {}
