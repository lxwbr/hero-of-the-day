use aws_config::SdkConfig;
use aws_sdk_dynamodb::{
    types::{AttributeValue, ReturnValue},
    Client,
};
use model::user::User;
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct UserRepository {
    client: Client,
    table_name: String,
}

impl UserRepository {
    pub fn new(shared_config: &SdkConfig) -> UserRepository {
        UserRepository {
            client: Client::new(shared_config),
            table_name: env::var("USER_TABLE").unwrap(),
        }
    }

    pub fn new_with_table_name(shared_config: &SdkConfig, table_name: String) -> UserRepository {
        UserRepository {
            client: Client::new(shared_config),
            table_name: env::var(table_name).unwrap(),
        }
    }

    pub async fn get(&self, email: String) -> Result<Option<User>, Error> {
        let response = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("email", AttributeValue::S(email))
            .send()
            .await?;
        match response.item() {
            None => Ok(None),
            Some(fields) => Ok(Some(User::from(fields))),
        }
    }

    pub async fn put(&self, user: &User) -> Result<(), Error> {
        let put_item = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .item("email", AttributeValue::S(user.email.to_string()));

        match user.last_login {
            Some(last_login) => {
                put_item
                    .item("last_login", AttributeValue::N(last_login.to_string()))
                    .return_values(ReturnValue::AllOld)
                    .send()
                    .await?;
            }
            None => {
                put_item.return_values(ReturnValue::AllOld).send().await?;
            }
        };
        Ok(())
    }

    pub async fn update_last_login(&self, email: String, last_login: u64) -> Result<(), Error> {
        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("email", AttributeValue::S(email))
            .update_expression("set last_login = :last_login")
            .expression_attribute_values(":last_login", AttributeValue::N(last_login.to_string()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn update_last_seen_release_notes(
        &self,
        email: String,
        last_seen_release_notes: String,
    ) -> Result<(), Error> {
        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("email", AttributeValue::S(email))
            .update_expression(
                "set update_last_seen_release_notes = :update_last_seen_release_notes",
            )
            .expression_attribute_values(
                ":update_last_seen_release_notes",
                AttributeValue::S(last_seen_release_notes.to_string()),
            )
            .send()
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<User>, Error> {
        let response = self
            .client
            .scan()
            .table_name(&self.table_name)
            .send()
            .await?;
        let heroes: Vec<User> = response.items().iter().map(User::from).collect();
        Ok(heroes)
    }
}
