use lambda::handler_fn;
use serde_json::{ Value, json };
use response::ok;
use model::user::User;

extern crate rusoto_core;
extern crate rusoto_dynamodb;

use rusoto_core::{Region};
use rusoto_dynamodb::{DynamoDbClient};
use repository::user::UserRepository;

mod error;
use error::UserPutError;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(event: Value) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());

    let repository = UserRepository::new(client);

    let user = User {
        email: event["email"].as_str().ok_or(UserPutError::NoEmailProvided)?.to_string(),
        last_login: None
    };

    repository.put(&user).await?;

    Ok(ok(json!(user).to_string()))
}
