use futures::{prelude::*};
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
    pub async fn usergroups_list(&self) -> Result<Vec<Usergroup>, Error> {
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
    pub async fn lookup_by_email(&self, email: String) -> Result<User, Error> {
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
        &self,
        usergroup_id: &String,
        user_ids: &Vec<String>,
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

    async fn look_up_user_ids_by_email(&self, schedule: &Schedule) -> Result<Vec<String>, Error> {
        let result = future::try_join_all(schedule.assignees.iter().map(|assignee|
            self.lookup_by_email(assignee.clone()).map_ok(|user| user.id)
        )).await;
        result
    }

    pub async fn usergroups_users_update_with_schedules(
        &self,
        schedules: Vec<Schedule>,
    ) -> Result<(), Error> {
        let usergroup_id_map: HashMap<String, String> = self.usergroups_list().map_ok(|usergroups| usergroups.into_iter().map(|a| (a.handle, a.id)).collect()).await?;

        let updates: Vec<(&String, Vec<String>)> = future::try_join_all(schedules.iter().map(|schedule| {
            self.look_up_user_ids_by_email(schedule).map_ok(|users| {
                match usergroup_id_map.get(&schedule.hero) {
                    Some(usergroup_id) => {
                        Some((usergroup_id, users))
                    }
                    None => {
                        println!("no usergroup id for {}", schedule.hero);
                        None
                    }
                }
            })
        })).await?.into_iter().flatten().collect();

        future::try_join_all(updates.iter().map(|(usergroup_id, user_ids)| {
            println!("Updating usergroup_id {}: user_ids: {:?}", usergroup_id, user_ids);
            self.usergroups_users_update(usergroup_id, user_ids)
        })).await?;

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