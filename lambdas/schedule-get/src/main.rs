use chrono::{DateTime, Utc};
use lambda_http::{run, service_fn, Error, Request, RequestExt, RequestPayloadExt};
use repository::schedule::ScheduleRepository;
use response::{bad_request, ok};
use serde::Deserialize;

fn to_epoch_seconds(string: &str) -> i64 {
    let date_time = DateTime::parse_from_rfc3339(string)
        .expect("`shift_start_time` has to be a rfc3339 string")
        .with_timezone(&Utc);

    date_time.timestamp()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    let shared_config = aws_config::load_from_env().await;
    let repository_ref = &ScheduleRepository::new(&shared_config);

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("hero") {
            Some(hero) => {
                let between = event.payload::<Payload>()?.map(|payload| {
                    (
                        to_epoch_seconds(payload.start_timestamp.as_str()),
                        to_epoch_seconds(payload.end_timestamp.as_str()),
                    )
                });
                let schedules = repository_ref.get(hero.into(), between).await?;
                ok(schedules)
            }
            _ => bad_request("Hero parameter missing".into()),
        }
    }))
    .await?;
    Ok(())
}

#[derive(Deserialize)]
struct Payload {
    start_timestamp: String,
    end_timestamp: String,
}
