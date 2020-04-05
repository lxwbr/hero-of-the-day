use model::hero::Hero;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemInput, UpdateItemInput};
use std::env;

use maplit::hashmap;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct HeroRepository<'a> {
    client: &'a DynamoDbClient,
    table_name: String,
}

impl HeroRepository<'_> {
    pub fn new(client: &DynamoDbClient) -> HeroRepository {
        HeroRepository {
            client,
            table_name: env::var("HERO_TABLE").unwrap(),
        }
    }

    pub async fn get(self, name: String) -> Result<Hero, Error> {
        let attribute_values = hashmap! {
            "name".to_owned() => AttributeValue {
                s: Some(name),
                ..Default::default()
            }
        };

        let get_item_input = GetItemInput {
            table_name: self.table_name,
            key: attribute_values,
            ..Default::default()
        };

        let hero: Hero = Hero::from_dynamo_item(
            self.client
                .get_item(get_item_input)
                .await?
                .item
                .expect("Expected to receive an item"),
        );

        Ok(hero)
    }

    pub async fn append_members(
        self,
        hero: String,
        members: Vec<String>,
    ) -> Result<Vec<String>, Error> {
        let key = hashmap! {
            "name".to_string() => AttributeValue {
                s: Some(hero.clone()),
                ..Default::default()
            }
        };

        let expression_attribute_values = hashmap! {
            ":m".to_string() => AttributeValue {
                ss: Some(members),
                ..Default::default()
            }
        };

        let update_item_input = UpdateItemInput {
            table_name: self.table_name,
            key,
            update_expression: Some("ADD members :m".to_string()),
            expression_attribute_values: Some(expression_attribute_values),
            return_values: Some("UPDATED_NEW".to_string()),
            ..Default::default()
        };

        let attributes = self
            .client
            .update_item(update_item_input)
            .await?
            .attributes
            .expect("Expected attributes from the UpdateItemInput.");

        if attributes.is_empty() {
            Ok(Vec::new())
        } else {
            let appended_members = attributes["members"].ss.as_ref().unwrap().to_vec();
            println!(
                "Following were added to the {} hero as members: {:?}",
                hero, appended_members
            );
            Ok(appended_members)
        }
    }
}
