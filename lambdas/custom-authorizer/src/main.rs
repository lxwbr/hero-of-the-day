use lambda::handler_fn;
use serde_json::{ Value, json };
use std::env;
use google_signin;
use model::user::User;
use repository::user::UserRepository;
extern crate rusoto_core;
extern crate rusoto_dynamodb;

use rusoto_core::{Region};
use rusoto_dynamodb::{DynamoDbClient};
use std::time::{SystemTime, UNIX_EPOCH};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(func);
    lambda::run(func).await?;
    Ok(())
}

async fn func(event: Value) -> Result<Value, Error> {
    let google_client_id_env = "GOOGLE_CLIENT_ID";
    let hosted_domain_env = "HOSTED_DOMAIN";

    let google_client_id = env::var(google_client_id_env).expect("Expected environment variable GOOGLE_CLIENT_ID not set");
    let hosted_domain = env::var(hosted_domain_env).expect("Expected environment variable HOSTED_DOMAIN not set");

    let mut client = google_signin::Client::new();
    client.audiences.push(google_client_id.clone());
    client.hosted_domains.push(hosted_domain.clone());

    // This will slice out the `Bearer ` part of the authorization token
    let id_token = &event["authorizationToken"].as_str().expect("Expected authorizationToken to be part of the event")[7..];
    let method_arn = event["methodArn"].as_str().expect("Expected methodArn to be part of the event");
    let id_info = client.verify(id_token).expect("Expected token to be valid");
    println!("Success! Signed-in as {:?}", id_info.email);
    logged_in(id_info.email.to_owned().expect("id_info should have the email field")).await?;

    check_user(method_arn.to_owned(), id_info).await
}

async fn check_user(method_arn: String, id_info: google_signin::IdInfo) -> Result<Value, Error> {
    let parts: Vec<&str> = method_arn.split("/").collect();
    let http_verb = parts[2];
    let resource = parts[3];

    let apply_policy = policy( id_info.sub, method_arn.clone());

    let value = if http_verb == "POST" || http_verb == "PUT" {
        if resource == "user" {
            apply_policy(Effect::Allow)
        } else {
            apply_policy(Effect::Deny)
        }
    } else {
        apply_policy(Effect::Allow)
    };

    Ok(value)
}

enum Effect {
    Allow,
    Deny
}

async fn logged_in(email: String) -> Result<User, Error> {
    let client = DynamoDbClient::new(Region::default());

    let repository = UserRepository::new(client);

    let user = User {
        email,
        last_login: Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
    };

    repository.put(&user).await?;

    Ok(user)
}

fn policy(principal_id: String, method_arn: String) -> impl Fn(Effect) -> Value {
    move |effect|
        json!({
            "principalId": principal_id,
            "policyDocument": {
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Sid": "FirstStatement",
                        "Action": "execute-api:Invoke",
                        "Effect": match effect {
                            Effect::Allow => "Allow",
                            Effect::Deny => "Deny"
                        },
                        "Resource": method_arn
                    }
                ]
            }
        })
}
