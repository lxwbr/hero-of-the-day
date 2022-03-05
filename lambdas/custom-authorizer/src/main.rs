use google_jwt_verify;
use lambda_runtime::{service_fn, LambdaEvent};
use model::user::User;
use repository::{hero::HeroRepository, user::UserRepository};
use rusoto_core::Region;
use rusoto_dynamodb::DynamoDbClient;
use serde_json::{json, Value};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use jsonwebtoken::{dangerous_insecure_decode};
use serde::{Serialize, Deserialize};
use azure_jwt::*;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,         // Optional. Issuer
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(event: LambdaEvent<Value>) -> Result<Value, Error> {
    // This will slice out the `Bearer ` part of the authorization token
    let id_token = &event.payload["authorizationToken"]
        .as_str()
        .expect("Expected authorizationToken to be part of the event")[7..];

    let method_arn = event.payload["methodArn"]
        .as_str()
        .expect("Expected methodArn to be part of the event");

    let token_data = dangerous_insecure_decode::<Claims>(&id_token)?;
    let dynamo_client = DynamoDbClient::new(Region::default());
    println!("Logging in with iss: {:?}", token_data.claims.iss);
    if token_data.claims.iss.contains("google") {
        let google_client_id_env = "GOOGLE_CLIENT_ID";

        let google_client_id = env::var(google_client_id_env)
            .expect("Expected environment variable GOOGLE_CLIENT_ID not set");

        let google_client = google_jwt_verify::Client::new(google_client_id.as_str());
        let verified_token = google_client.verify_id_token(id_token).expect("Expected token to be valid");

        let email = verified_token.get_payload().get_email();
        println!("Signed-in as {:?}", email);

        logged_in(
            &dynamo_client,
            email.to_owned(),
        )
            .await?;

        check_user(&dynamo_client, method_arn.to_owned(), Info {
            sub: verified_token.get_claims().get_subject(),
            email
        }).await
    } else {
        let aud = env::var("MS_CLIENT_ID")
            .expect("Expected environment variable MS_CLIENT_ID not set");
        let mut az_auth = AzureAuth::new(aud).unwrap();
        let token = az_auth.validate_token(id_token).expect("Expected valid token");

        println!("Signed-in as {:?}", token.claims.preferred_username);

        logged_in(
            &dynamo_client,
            token.claims.preferred_username
                .to_owned()
                .expect("token claims should have the preferred_username field"),
        )
            .await?;

        check_user(&dynamo_client, method_arn.to_owned(), Info {
            sub: token.claims.sub,
            email: token.claims.preferred_username.expect("token claims should have the preferred_username field")
        }).await
    }
}

struct Info {
    sub: String,
    email: String
}

async fn check_user(
    client: &DynamoDbClient,
    method_arn: String,
    info: Info,
) -> Result<Value, Error> {
    let sub = info.sub;
    let parts: Vec<&str> = method_arn.split("/").collect();
    let http_verb = parts[2];
    let resource = parts[3];
    let sub_resource = parts[4];

    let apply_policy = policy(sub.clone(), method_arn.clone());

    let value = if http_verb == "POST" || http_verb == "PUT" {
        if resource == "user" {
            apply_policy(Effect::Allow)
        } else {
            let repository = HeroRepository::new(client);
            let hero = repository.get(sub_resource.to_string()).await?;
            let email = info.email;
            println!("email: {} in {:?}", email, hero.members);
            if hero.members.contains(&email) {
                println!("ALLOW");
                apply_policy(Effect::Allow)
            } else {
                println!("DENY");
                apply_policy(Effect::Deny)
            }
        }
    } else {
        apply_policy(Effect::Allow)
    };

    Ok(value)
}

enum Effect {
    Allow,
    Deny,
}

async fn logged_in(client: &DynamoDbClient, email: String) -> Result<User, Error> {
    let repository = UserRepository::new(client);

    let user = User {
        email,
        last_login: Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()),
    };

    repository.put(&user).await?;

    Ok(user)
}

fn policy(principal_id: String, method_arn: String) -> impl Fn(Effect) -> Value {
    move |effect| {
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
}
