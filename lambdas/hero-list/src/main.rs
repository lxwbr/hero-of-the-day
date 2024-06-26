use lambda_http::{run, service_fn, Error, Request};
use model::hero::Hero;
use repository::hero::HeroRepository;
use response::ok;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .flatten_event(true)
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let shared_config = aws_config::load_from_env().await;
    let repository_ref = &HeroRepository::new(&shared_config);

    run(service_fn(move |_: Request| async move {
        tracing::info!("Fetching heroes...");
        let heroes: Vec<Hero> = repository_ref.list().await?;
        tracing::info!("Fetched {} heroes.", heroes.len());
        ok(heroes)
    }))
    .await?;
    Ok(())
}
