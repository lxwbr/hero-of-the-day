use aws_config::SdkConfig;
use aws_sdk_dynamodb::{
    types::{AttributeValue, ReturnValue},
    Client,
};
use model::hero::Hero;
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct HeroRepository {
    client: Client,
    table_name: String,
}

pub enum UpdateOperation {
    Add,
    Delete,
}

impl HeroRepository {
    pub fn new(shared_config: &SdkConfig) -> HeroRepository {
        HeroRepository {
            client: Client::new(shared_config),
            table_name: env::var("HERO_TABLE").unwrap(),
        }
    }

    pub fn new_with_table_name(shared_config: &SdkConfig, table_name: String) -> HeroRepository {
        HeroRepository {
            client: Client::new(shared_config),
            table_name: env::var(table_name).unwrap(),
        }
    }

    pub async fn get(&self, name: String) -> Result<Hero, Error> {
        let response = self
            .client
            .get_item()
            .key("name", AttributeValue::S(name))
            .table_name(&self.table_name)
            .send()
            .await?;
        let hero: Hero = Hero::try_from(response.item().expect("hero not found"))?;
        Ok(hero)
    }

    pub async fn list(&self) -> Result<Vec<Hero>, Error> {
        let response = self
            .client
            .scan()
            .table_name(&self.table_name)
            .send()
            .await?;
        let heroes: Vec<Hero> = response
            .items()
            .iter()
            .filter_map(|item| match Hero::try_from(item) {
                Ok(item) => Some(item),
                Err(err) => {
                    eprintln!("Failed to parse item: {}", err.to_string());
                    None
                }
            })
            .collect();
        Ok(heroes)
    }

    pub async fn put(&self, hero: &Hero) -> Result<(), Error> {
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("name", AttributeValue::S(hero.name.to_string()))
            .item("members", AttributeValue::Ss(hero.members.to_owned()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn update_members(
        &self,
        hero: String,
        members: Vec<String>,
        operation: UpdateOperation,
    ) -> Result<Vec<String>, Error> {
        let update_expression = match operation {
            UpdateOperation::Add => "ADD members :m",
            UpdateOperation::Delete => "DELETE members :m",
        };

        let attributes = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("name", AttributeValue::S(hero.clone()))
            .expression_attribute_values(":m", AttributeValue::Ss(members))
            .update_expression(update_expression)
            .return_values(ReturnValue::UpdatedNew)
            .send()
            .await?
            .attributes
            .expect("Expected attributes from the UpdateItemInput.");

        if attributes.is_empty() {
            Ok(Vec::new())
        } else {
            let appended_members = attributes["members"].as_ss().unwrap().to_vec();
            println!(
                "Following were added to the {} hero as members: {:?}",
                hero, appended_members
            );
            Ok(appended_members)
        }
    }

    pub async fn delete(&self, hero: String) -> Result<(), Error> {
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("name", AttributeValue::S(hero.clone()))
            .send()
            .await?;

        Ok(())
    }
}
