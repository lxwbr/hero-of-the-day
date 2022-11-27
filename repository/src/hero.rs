use aws_config::SdkConfig;
use model::hero::Hero;
use aws_sdk_dynamodb::{Client, model::{AttributeValue, ReturnValue}};
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct HeroRepository {
    client: Client,
    table_name: String
}

impl HeroRepository {
    pub fn new(shared_config: &SdkConfig) -> HeroRepository {
        HeroRepository {
            client: Client::new(&shared_config),
            table_name: env::var("HERO_TABLE").unwrap()
        }
    }

    pub fn new_with_table_name(shared_config: &SdkConfig, table_name: String) -> HeroRepository {
        HeroRepository {
            client: Client::new(&shared_config),
            table_name: env::var(table_name).unwrap()
        }
    }

    pub async fn get(&self, name: String) -> Result<Hero, Error> {
        let response = self.client
            .get_item()
            .key("name", AttributeValue::S(name))
            .table_name(&self.table_name)
            .send()
            .await?;
        let hero: Hero = Hero::from_dynamo_item(response.item().expect("hero not found"));
        Ok(hero)
    }

    pub async fn list(&self) -> Result<Vec<Hero>, Error> {
        let response = self.client
            .scan()
            .table_name(&self.table_name)
            .send()
            .await?;
        let heroes: Vec<Hero> = response
            .items()
            .unwrap_or_default()
            .into_iter()
            .map(Hero::from_dynamo_item)
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

    pub async fn append_members(
        &self,
        hero: String,
        members: Vec<String>,
    ) -> Result<Vec<String>, Error> {
        let attributes = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("name", AttributeValue::S(hero.clone()))
            .expression_attribute_values(":m", AttributeValue::Ss(members))
            .update_expression("ADD members :m")
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
}