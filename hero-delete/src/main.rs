use lambda_http::{run, service_fn, Error, Request, RequestExt};
use repository::{hero::HeroRepository, schedule::ScheduleRepository};
use model::hero::Hero;
use response::{ok, bad_request};
use serde::{Deserialize};

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

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("hero") {
            Some(name) => {
                schedule_repository_ref.delete(name.to_string()).await?;
                hero_repository_ref.delete(name.to_string()).await?;
                ok(())
            },
            _ => bad_request("Expected hero".into())
        }
    })).await?;
    Ok(())
}
