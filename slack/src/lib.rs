use aws_sdk_ssm::Client as SsmClient;
use futures::prelude::*;
use model::schedule::Schedule;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use thiserror::Error;

type Result<T> = std::result::Result<T, SlackError>;

#[derive(Error, Debug)]
pub enum SlackError {
    #[error("Could not update user group users.")]
    UserGroupUsersUpdateError,
    #[error("Could not lookup by email. {0}")]
    UsersLookupByEmailError(String),
    #[error("Could not list user groups")]
    UserGroupsList,
    #[error("Could not create user group.")]
    CreateUserGroupError,
    #[error("Could not post message.")]
    PostMessageError,
    #[error("Could not get Slack token.")]
    GetSlackTokenError,
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
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

#[derive(Deserialize, Debug)]
struct PostMessageResponse {
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
    pub async fn usergroups_list(&self) -> Result<Vec<Usergroup>> {
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
            Err(SlackError::UserGroupsList)
        }
    }

    /// Resolves Slack user id by using user's e-mail address.
    pub async fn lookup_by_email(&self, email: String) -> Result<User> {
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
            .map_err(|_| SlackError::UsersLookupByEmailError(email.clone()))
            .await?
            .json()
            .map_err(|_| SlackError::UsersLookupByEmailError(email.clone()))
            .await?;
        if result.ok {
            Ok(result.user)
        } else {
            Err(SlackError::UsersLookupByEmailError(email))
        }
    }

    pub async fn usergroups_users_update(
        &self,
        usergroup_id: &String,
        user_ids: &[String],
    ) -> Result<()> {
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
            Err(SlackError::UserGroupUsersUpdateError)
        }
    }

    pub async fn create_usergroup(&self, usergroup_name: &String) -> Result<()> {
        let url = format!(
            "https://slack.com/api/usergroups.create?token={}&pretty=1&name={}",
            self.token, usergroup_name
        );
        println!("url: {}", url);
        let result: UsergroupsUsersUpdateResponse =
            self.client.post(url.as_str()).send().await?.json().await?;
        if result.ok {
            Ok(())
        } else {
            Err(SlackError::CreateUserGroupError)
        }
    }

    async fn look_up_user_ids_by_email(&self, schedule: &Schedule) -> Result<Vec<String>> {
        future::try_join_all(schedule.assignees.iter().map(|assignee| {
            self.lookup_by_email(assignee.clone())
                .map_ok(|user| user.id)
        }))
        .await
        .map_err(|err| {
            println!("{}", err);
            err
        })
    }

    pub async fn usergroups_users_update_with_schedules(
        &self,
        schedules: Vec<Schedule>,
    ) -> Result<()> {
        let usergroup_id_map: HashMap<String, String> = self
            .usergroups_list()
            .map_ok(|usergroups| usergroups.into_iter().map(|a| (a.handle, a.id)).collect())
            .await?;

        let updates: Vec<Option<(&String, Vec<String>)>> =
            future::join_all(schedules.iter().map(|schedule| {
                self.look_up_user_ids_by_email(schedule).map_ok(|users| {
                    match usergroup_id_map.get(&schedule.hero) {
                        Some(usergroup_id) => Some((usergroup_id, users)),
                        None => {
                            println!("no usergroup id for {}", schedule.hero);
                            None
                        }
                    }
                })
            }))
            .await
            .into_iter()
            .flatten()
            .collect();

        let filtered: Vec<(&String, Vec<String>)> = updates.into_iter().flatten().collect();

        future::join_all(filtered.iter().map(|(usergroup_id, user_ids)| {
            println!(
                "Updating usergroup_id {}: user_ids: {:?}",
                usergroup_id, user_ids
            );
            self.usergroups_users_update(usergroup_id, user_ids.as_ref())
        }))
        .await;

        Ok(())
    }

    pub async fn post_message(
        &self,
        channel_id: &String,
        hero: &String,
        assignees: Vec<String>,
    ) -> Result<()> {
        let url = format!(
            "https://slack.com/api/chat.postMessage?token={}&channel={}&text={}:%20{}&pretty=1",
            self.token,
            channel_id,
            hero,
            assignees.join(",%20")
        );
        let result: PostMessageResponse =
            self.client.post(url.as_str()).send().await?.json().await?;
        if result.ok {
            Ok(())
        } else {
            Err(SlackError::PostMessageError)
        }
    }
}

/// Retrieves Slack application token from SSM.
pub async fn get_slack_token() -> Result<String> {
    let shared_config = aws_config::load_from_env().await;
    let client = SsmClient::new(&shared_config);
    let token = client
        .get_parameter()
        .name(env::var("SLACK_TOKEN_PARAMETER").map_err(|_| SlackError::GetSlackTokenError)?)
        .send()
        .map_err(|_| SlackError::GetSlackTokenError)
        .await?
        .parameter
        .expect("Slack token not found as an SSM parameter.")
        .value
        .expect("Slack token needs to non-empty.");

    Ok(token)
}
