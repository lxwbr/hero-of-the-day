use aws_lambda_events::apigw::{
    ApiGatewayCustomAuthorizerPolicy, ApiGatewayCustomAuthorizerRequest,
    ApiGatewayCustomAuthorizerResponse, IamPolicyStatement,
};
use azure_jwt::*;
use jsonwebtoken::dangerous_insecure_decode;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use model::time::secs_now;
use repository::{hero::HeroRepository, user::UserRepository};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String, // Optional. Issuer
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // required to enable CloudWatch error logging by the runtime
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let shared_config = aws_config::load_from_env().await;
    let hero_repository_ref = &HeroRepository::new(&shared_config);
    let user_repository_ref = &UserRepository::new(&shared_config);

    run(service_fn(
        move |event: LambdaEvent<ApiGatewayCustomAuthorizerRequest>| async move {
            tracing::info!("Logging works");

            // This will slice out the `Bearer ` part of the authorization token
            let id_token = &event
                .payload
                .authorization_token
                .expect("missing authorization_token")[7..];

            let method_arn = &event.payload.method_arn.expect("missing method_arn");

            let token_data = dangerous_insecure_decode::<Claims>(id_token)?;

            tracing::info!("Logging in with iss: {:?}", token_data.claims.iss);

            let aud = env::var("MS_CLIENT_ID")
                .expect("Expected environment variable MS_CLIENT_ID not set");

            let mut az_auth = AzureAuth::new(aud).expect("Failed to create AzureAuth");
            match az_auth.validate_token(id_token) {
                Ok(token) => {
                    tracing::info!("Signed-in as {:?}", token.claims.preferred_username);

                    logged_in(
                        user_repository_ref,
                        token
                            .claims
                            .preferred_username
                            .to_owned()
                            .expect("token claims should have the preferred_username field"),
                    )
                    .await?;

                    let policy = check_user(
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
                    .await;
                    tracing::info!("Policy: {:?}", policy);
                    policy
                }
                Err(err) => {
                    tracing::error!("Error validating token: {:?}", err);
                    Ok(policy(None, method_arn.clone(), Some(err.to_string()))(
                        Effect::Deny,
                    ))
                }
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
) -> Result<ApiGatewayCustomAuthorizerResponse, Error> {
    let sub = info.sub;
    let parts: Vec<&str> = method_arn.split('/').collect();
    let http_verb = parts[2];
    let resource = parts[3];
    let sub_resource = parts[4];

    let apply_policy = policy(Some(sub.clone()), method_arn.clone(), None);

    let value = if http_verb == "POST" || http_verb == "PUT" {
        if resource == "user" {
            tracing::info!("ALLOW POST and PUT on user");
            apply_policy(Effect::Allow)
        } else {
            let email = info.email;
            if http_verb == "PUT" {
                tracing::info!("ALLOW PUT");
                apply_policy(Effect::Allow)
            } else {
                let hero = hero_repository_ref.get(sub_resource.to_string()).await?;
                tracing::info!("email: {} in {:?}", email, hero.members);
                if hero.members.contains(&email) {
                    tracing::info!("ALLOW");
                    apply_policy(Effect::Allow)
                } else {
                    tracing::info!("DENY");
                    apply_policy(Effect::Deny)
                }
            }
        }
    } else {
        tracing::info!("ALLOW GET and DELETE");
        apply_policy(Effect::Allow)
    };

    Ok(value)
}

enum Effect {
    Allow,
    Deny,
}

async fn logged_in(repository: &UserRepository, email: String) -> Result<(), Error> {
    repository.update_last_login(email, secs_now()).await?;
    Ok(())
}

fn policy(
    principal_id: Option<String>,
    method_arn: String,
    context: Option<String>,
) -> impl Fn(Effect) -> ApiGatewayCustomAuthorizerResponse {
    move |effect| ApiGatewayCustomAuthorizerResponse {
        principal_id: principal_id.clone(),
        policy_document: {
            ApiGatewayCustomAuthorizerPolicy {
                version: Some("2012-10-17".to_string()),
                statement: vec![IamPolicyStatement {
                    action: vec!["execute-api:Invoke".to_string()],
                    effect: Some(match effect {
                        Effect::Allow => "Allow".to_string(),
                        Effect::Deny => "Deny".to_string(),
                    }),
                    resource: vec![method_arn.clone()],
                }],
            }
        },
        context: json!(context),
        usage_identifier_key: None,
    }
}
