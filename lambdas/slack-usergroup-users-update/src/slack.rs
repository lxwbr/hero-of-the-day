use reqwest;
use rusoto_core::Region;
use rusoto_ssm::{GetParameterRequest, Ssm, SsmClient};
use serde::Deserialize;
use std::env;
use std::fmt;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug)]
pub enum SlackUsergroupUsersUpdateError {
    NotOk,
}

impl std::error::Error for SlackUsergroupUsersUpdateError {}

impl fmt::Display for SlackUsergroupUsersUpdateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SlackUsergroupUsersUpdateError::NotOk => {
                write!(f, "Slack response field `ok` is false!")
            }
        }
    }
}

pub struct Client {
    client: reqwest::Client,
    token: String,
}

#[derive(Deserialize, Debug)]
pub struct Usergroup {
    pub id: String,
    pub handle: String,
}

#[derive(Deserialize, Debug)]
struct UsergroupsListResponse {
    ok: bool,
    usergroups: Vec<Usergroup>,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: String,
}

#[derive(Deserialize, Debug)]
struct UsersLookupByEmailResponse {
    ok: bool,
    user: User,
}

#[derive(Deserialize, Debug)]
struct UsergroupsUsersUpdateResponse {
    ok: bool
}

impl Client {
    pub fn new(token: String) -> Client {
        Client {
            client: reqwest::Client::new(),
            token,
        }
    }

    /// Lists all Slack groups in order to have an id to handle relation.
    pub async fn usergroups_list(self) -> Result<Vec<Usergroup>, Error> {
        let result: UsergroupsListResponse = self
            .client
            .get(
                format!(
                    "https://slack.com/api/usergroups.list?token={}&pretty=1",
                    self.token
                )
                .as_str(),
            )
            .send()
            .await?
            .json()
            .await?;
        if result.ok {
            Ok(result.usergroups)
        } else {
            Err(Box::new(SlackUsergroupUsersUpdateError::NotOk))
        }
    }

    /// Resolves Slack user id by using user's e-mail address.
    pub async fn lookup_by_email(self, email: String) -> Result<User, Error> {
        let result: UsersLookupByEmailResponse = self
            .client
            .get(
                format!(
                    "https://slack.com/api/users.lookupByEmail?token={}&pretty=1&email={}",
                    self.token, email
                )
                .as_str(),
            )
            .send()
            .await?
            .json()
            .await?;
        if result.ok {
            Ok(result.user)
        } else {
            Err(Box::new(SlackUsergroupUsersUpdateError::NotOk))
        }
    }

    pub async fn usergroups_users_update(self, usergroup_id: String, user_ids: Vec<String>) -> Result<(), Error> {
        let url = format!(
            "https://slack.com/api/usergroups.users.update?token={}&pretty=1&usergroup={}&users={}",
            self.token, usergroup_id, user_ids.join(",")
        );
        println!("url: {}", url);
        let result: UsergroupsUsersUpdateResponse = self
            .client
            .post(
                url
                .as_str(),
            )
            .send()
            .await?
            .json()
            .await?;
        if result.ok {
            Ok(())
        } else {
            Err(Box::new(SlackUsergroupUsersUpdateError::NotOk))
        }
    }
}

/// Retrieves Slack application token from SSM.
pub async fn get_slack_token() -> Result<String, Error> {
    let client = SsmClient::new(Region::default());
    let get_parameter_request = GetParameterRequest {
        name: env::var("SLACK_TOKEN_PARAMETER")?,
        ..Default::default()
    };
    let token = client
        .get_parameter(get_parameter_request)
        .await?
        .parameter
        .expect("Slack token not found as an SSM parameter.")
        .value
        .expect("Slack token needs to non-empty.");

    Ok(token)
}
