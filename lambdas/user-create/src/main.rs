use lambda_http::{run, service_fn, Error, Request, RequestExt};
use model::user::User;
use repository::user::UserRepository;
use response::{bad_request, ok};

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
            Some(email) => {
                let user = User {
                    email: email.into(),
                    last_login: None,
                };
                ok(repository_ref.put(&user).await?)
            }
            _ => bad_request("Expected email".into()),
        }
    }))
    .await?;
    Ok(())
}
