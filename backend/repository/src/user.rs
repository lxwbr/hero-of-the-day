use std::env;
use rusoto_dynamodb::{DynamoDb, AttributeValue, DynamoDbClient, PutItemInput};
use model::user::User;
use maplit::hashmap;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct UserRepository {
    client: DynamoDbClient
}


impl UserRepository {
    pub fn new(client: DynamoDbClient) -> UserRepository {
        UserRepository { client }
    }

    pub async fn put(self, user: &User) -> Result<&User, Error> {
        let mut item = hashmap!{
            "email".to_owned() => AttributeValue {
                s: Some(user.email.clone()),
                ..Default::default()
            }
        };

        if let Some(last_login) = user.last_login {
            item.insert("last_login".to_string(), AttributeValue {
                n: Some(last_login.to_string()),
                ..Default::default()
            });
        }

        let put_item_input = PutItemInput {
            table_name: env::var("USER_TABLE")?,
            item,
            return_values: Some("ALL_OLD".to_string()),
            ..Default::default()
        };

        self.client.put_item(put_item_input).await?;

        Ok(user)
    }
}
