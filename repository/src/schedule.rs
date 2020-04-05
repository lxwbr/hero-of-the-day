use std::env;
use rusoto_dynamodb::{DynamoDb, AttributeValue, DynamoDbClient, QueryInput, UpdateItemInput};
use model::schedule::Schedule;
use maplit::hashmap;

#[path = "error.rs"] mod error;
use error::RepositoryError;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct ScheduleRepository<'a> {
    client: &'a DynamoDbClient,
    table_name: String
}


impl ScheduleRepository <'_> {
    pub fn new(client: &DynamoDbClient) -> ScheduleRepository {
        ScheduleRepository { client, table_name: env::var("SCHEDULE_TABLE").unwrap() }
    }

    pub async fn get(self, hero: String) -> Result<Vec<Schedule>, Error> {
        let attribute_values = hashmap!{
            ":hero".to_owned() => AttributeValue {
                s: Some(hero),
                ..Default::default()
            }
        };

        let query_input = QueryInput {
            table_name: self.table_name,
            key_condition_expression: Some("hero = :hero".to_string()),
            expression_attribute_values: Some(attribute_values),
            ..Default::default()
        };

        let schedules: Vec<Schedule> = self.client.query(query_input).await?.items
            .ok_or(RepositoryError::NoneScan)?
            .into_iter()
            .map(Schedule::from_dynamo_item)
            .collect();

        Ok(schedules)
    }

    pub async fn append_assignee(self, hero: String, shift_start_time: i64, assignees: Vec<String>) -> Result<Schedule, Error> {
        let key = hashmap!{
            "hero".to_string() => AttributeValue {
                s: Some(hero),
                ..Default::default()
            },
            "shift_start_time".to_owned() => AttributeValue {
                n: Some(shift_start_time.to_string()),
                ..Default::default()
            }
        };

        let expression_attribute_values = hashmap! {
            ":a".to_string() => AttributeValue {
                ss: Some(assignees),
                ..Default::default()
            }
        };

        let update_item_input = UpdateItemInput {
            table_name: self.table_name,
            key,
            update_expression: Some("ADD assignees :a".to_string()),
            expression_attribute_values: Some(expression_attribute_values),
            return_values: Some("ALL_NEW".to_string()),
            ..Default::default()
        };

        let attributes = self.client.update_item(update_item_input).await?.attributes.expect("Expected attributes from the UpdateItemInput.");

        Ok(Schedule::from_dynamo_item(attributes))
    }
}
