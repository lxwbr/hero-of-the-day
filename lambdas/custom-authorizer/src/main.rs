use lambda::lambda;
use serde_json::{ Value, json };
use std::env;
use google_signin;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[lambda]
#[tokio::main]
async fn main(event: Value) -> Result<Value, Error> {
    let google_client_id_env = "GOOGLE_CLIENT_ID";
    let hosted_domain_env = "HOSTED_DOMAIN";

    let google_client_id = env::var(google_client_id_env).expect("Expected environment variable GOOGLE_CLIENT_ID not set");
    let hosted_domain = env::var(hosted_domain_env).expect("Expected environment variable HOSTED_DOMAIN not set");

    let mut client = google_signin::Client::new();
    client.audiences.push(google_client_id.clone());
    client.hosted_domains.push(hosted_domain.clone());

    let id_info = client.verify(&event["authorizationToken"].as_str().expect("Expected authorizationToken to be part of the event")).expect("Expected token to be valid");
    println!("Success! Signed-in as {}", id_info.sub);
    let response = json!({
        "principalId": id_info.sub,
        "policyDocument": {
        "Version": "2012-10-17",
        "Statement": [
            {
                "Sid": "FirstStatement",
                "Action": "execute-api:Invoke",
                "Effect": "Allow",
                "Resource": event["methodArn"].as_str().expect("Expected methodArn to be part of the event")
            }
        ]
        }
    });
    Ok(response)
}
