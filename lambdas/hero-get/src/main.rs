mod error;

use error::HeroGetError;
use lambda_runtime::{handler_fn, Context};
use repository::hero::HeroRepository;
use response::ok;
use rusoto_core::Region;
use rusoto_dynamodb::DynamoDbClient;
use serde_json::{json, Value};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(event: Value, _: Context) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());
    let repository = HeroRepository::new(&client);

    let hero = event["pathParameters"]["hero"]
        .as_str()
        .ok_or(HeroGetError::HeroParameterMissing)?;

    let hero = repository.get(hero.to_string()).await?;

    Ok(ok(json!(hero).to_string()))
}
