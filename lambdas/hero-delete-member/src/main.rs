use email_address::EmailAddress;
use lambda_http::{run, service_fn, Error, Request, RequestExt};
use repository::hero::HeroRepository;
use response::{bad_request, ok};
use std::str::FromStr;

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
            Some(name) => match event.path_parameters().first("member") {
                Some(member) => match EmailAddress::from_str(member) {
                    Ok(member) => {
                        let members = repository_ref
                            .update_members(
                                name.to_string(),
                                vec![member],
                                repository::hero::UpdateOperation::Delete,
                            )
                            .await?;
                        ok(members)
                    }
                    Err(err) => {
                        bad_request(format!("Member {} was not a proper email: {}", member, err))
                    }
                },
                _ => bad_request("Expected member".into()),
            },
            _ => bad_request("Expected hero".into()),
        }
    }))
    .await?;
    Ok(())
}
