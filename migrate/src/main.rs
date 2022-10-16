use lambda_runtime::{run, service_fn, LambdaEvent, Error};
use repository::{hero::HeroRepository, user::UserRepository};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Request {}

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
    let old_hero_repository_ref = &HeroRepository::new_with_table_name(&shared_config, "OLD_HERO_TABLE".to_string());
    let user_repository_ref = &UserRepository::new(&shared_config);
    let old_user_repository_ref = &UserRepository::new_with_table_name(&shared_config, "OLD_USER_TABLE".to_string());

    run(service_fn(move |_: LambdaEvent<Request>| async move {
        let old_heroes = old_hero_repository_ref.list().await?;
        for hero in old_heroes {
            hero_repository_ref.put(&hero).await?;
        };

        let old_users = old_user_repository_ref.list().await?;
        for user in old_users {
            user_repository_ref.put(&user).await?;
        };

        Ok::<(), Error>(())
    })).await?;
    Ok(())
}