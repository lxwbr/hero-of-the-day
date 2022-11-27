use futures::{prelude::*, stream::futures_unordered::FuturesUnordered};
use model::schedule::Schedule;
use reqwest;
use aws_sdk_ssm::{Client as SsmClient};
use serde::Deserialize;
use std::collections::HashMap;
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
    ok: bool,
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

    pub async fn usergroups_users_update(
        self,
        usergroup_id: String,
        user_ids: Vec<String>,
    ) -> Result<(), Error> {
        let url =
            format!(
            "https://slack.com/api/usergroups.users.update?token={}&pretty=1&usergroup={}&users={}",
            self.token, usergroup_id, user_ids.join(",")
        );
        println!("url: {}", url);
        let result: UsergroupsUsersUpdateResponse =
            self.client.post(url.as_str()).send().await?.json().await?;
        if result.ok {
            Ok(())
        } else {
            Err(Box::new(SlackUsergroupUsersUpdateError::NotOk))
        }
    }

    pub async fn usergroups_users_update_with_schedules(
        self,
        schedules: Vec<Schedule>,
    ) -> Result<(), Error> {
        let token = self.token.clone();

        let mut usergroup_id_map = HashMap::new();
        for usergroup in self.usergroups_list().await? {
            usergroup_id_map.insert(usergroup.handle, usergroup.id);
        }

        for schedule in schedules {
            match usergroup_id_map.get(&schedule.hero) {
                Some(usergroup_id) => {
                    let mut user_ids = Vec::new();
                    let mut user_id_results = schedule
                        .assignees
                        .iter()
                        .map(|assignee| {
                            Client::new(token.clone()).lookup_by_email(assignee.clone())
                        })
                        .collect::<FuturesUnordered<_>>();

                    while let Some(user_result) = user_id_results.next().await {
                        match user_result {
                            Ok(user) => user_ids.push(user.id),
                            Err(e) => println!("Got error back: {}", e),
                        }
                    }

                    Client::new(token.clone())
                        .usergroups_users_update(usergroup_id.clone(), user_ids.clone())
                        .await?;

                    println!("{}: {:?}", usergroup_id, user_ids);
                }
                None => {
                    println!("no usergroup id");
                }
            }
        }
        Ok(())
    }
}

/// Retrieves Slack application token from SSM.
pub async fn get_slack_token() -> Result<String, Error> {
    let shared_config = aws_config::load_from_env().await;
    let client = SsmClient::new(&shared_config);
    let token = client
        .get_parameter()
        .name(env::var("SLACK_TOKEN_PARAMETER")?)
        .send()
        .await?
        .parameter
        .expect("Slack token not found as an SSM parameter.")
        .value
        .expect("Slack token needs to non-empty.");

    Ok(token)
}