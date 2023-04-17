use azure_jwt::*;
use google_jwt_verify;
use jsonwebtoken::dangerous_insecure_decode;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use model::user::User;
use repository::{hero::HeroRepository, user::UserRepository};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::log::info;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String, // Optional. Issuer
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct APIGatewayCustomAuthorizerRequest {
    authorization_token: String,
    method_arn: String,
}

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
    let user_repository_ref = &UserRepository::new(&shared_config);

    run(service_fn(
        move |event: LambdaEvent<APIGatewayCustomAuthorizerRequest>| async move {
            // This will slice out the `Bearer ` part of the authorization token
            let id_token = &event.payload.authorization_token[7..];

            let method_arn = &event.payload.method_arn;

            let token_data = dangerous_insecure_decode::<Claims>(&id_token)?;

            info!("Logging in with iss: {:?}", token_data.claims.iss);

            if token_data.claims.iss.contains("google") {
                let google_client_id_env = "GOOGLE_CLIENT_ID";

                let google_client_id = env::var(google_client_id_env)
                    .expect("Expected environment variable GOOGLE_CLIENT_ID not set");

                let google_client = google_jwt_verify::Client::new(google_client_id.as_str());
                let verified_token = google_client
                    .verify_id_token(id_token)
                    .expect("Expected token to be valid");

                let email = verified_token.get_payload().get_email();
                info!("Signed-in as {:?}", email);

                logged_in(user_repository_ref, email.to_owned()).await?;

                check_user(
                    hero_repository_ref,
                    method_arn.to_owned(),
                    Info {
                        sub: verified_token.get_claims().get_subject(),
                        email,
                    },
                )
                .await
            } else {
                let aud = env::var("MS_CLIENT_ID")
                    .expect("Expected environment variable MS_CLIENT_ID not set");

                let mut az_auth = AzureAuth::new(aud).unwrap();
                let token = az_auth
                    .validate_token(id_token)
                    .expect("Expected valid token");

                info!("Signed-in as {:?}", token.claims.preferred_username);

                logged_in(
                    user_repository_ref,
                    token
                        .claims
                        .preferred_username
                        .to_owned()
                        .expect("token claims should have the preferred_username field"),
                )
                .await?;

                check_user(
                    hero_repository_ref,
                    method_arn.to_owned(),
                    Info {
                        sub: token.claims.sub,
                        email: token
                            .claims
                            .preferred_username
                            .expect("token claims should have the preferred_username field"),
                    },
                )
                .await
            }
        },
    ))
    .await?;
    Ok(())
}

struct Info {
    sub: String,
    email: String,
}

async fn check_user(
    hero_repository_ref: &HeroRepository,
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
            let email = info.email;
            if http_verb == "PUT" {
                apply_policy(Effect::Allow)
            } else {
                let hero = hero_repository_ref.get(sub_resource.to_string()).await?;
                info!("email: {} in {:?}", email, hero.members);
                if hero.members.contains(&email) {
                    info!("ALLOW");
                    apply_policy(Effect::Allow)
                } else {
                    info!("DENY");
                    apply_policy(Effect::Deny)
                }
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

async fn logged_in(repository: &UserRepository, email: String) -> Result<(), Error> {
    let user = User {
        email,
        last_login: Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()),
    };

    repository.put(&user).await?;

    Ok(())
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
