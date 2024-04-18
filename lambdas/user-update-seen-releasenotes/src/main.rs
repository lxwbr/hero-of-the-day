use lambda_http::{run, service_fn, Error, Request, RequestExt, RequestPayloadExt};
use repository::user::UserRepository;
use response::{bad_request, ok};
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
    let repository_ref = &UserRepository::new(&shared_config);

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("email") {
            Some(email) => match event.payload::<Payload>()? {
                Some(Payload { release_notes }) => ok(repository_ref
                    .update_last_seen_release_notes(email.to_string(), release_notes)
                    .await?),
                None => bad_request("Could not parse JSON payload for schedule update".into()),
            },
            _ => bad_request("Expected email".into()),
        }
    }))
    .await?;
    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct Payload {
    release_notes: String,
}
