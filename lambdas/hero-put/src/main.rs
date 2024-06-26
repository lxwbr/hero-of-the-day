use lambda_http::{run, service_fn, Error, Request, RequestExt, RequestPayloadExt};
use model::hero::Hero;
use repository::hero::HeroRepository;
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
    let repository_ref = &HeroRepository::new(&shared_config);

    run(service_fn(move |event: Request| async move {
        match event.path_parameters().first("hero") {
            Some(name) => {
                match event.payload::<Payload>()? {
                    Some(payload) => {
                        let hero = Hero {
                            name: name.to_string(),
                            members: payload.members,
                            channel: None,
                        };
                        repository_ref.put(&hero).await?;
                        // slack::Client::new(slack::get_slack_token().await?).create_usergroup(&name.to_string());
                        ok(())
                    }
                    None => bad_request("Could not parse JSON payload for schedule update".into()),
                }
            }
            _ => bad_request("Expected hero".into()),
        }
    }))
    .await?;
    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct Payload {
    members: Vec<String>,
}
