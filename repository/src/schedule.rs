use std::env;
use rusoto_dynamodb::{DynamoDb, AttributeValue, DynamoDbClient, QueryInput, UpdateItemInput, DeleteItemInput };
use model::schedule::Schedule;
use maplit::hashmap;

#[path = "error.rs"] mod error;
use error::RepositoryError;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct ScheduleRepository<'a> {
    client: &'a DynamoDbClient,
    table_name: String
}

pub enum Operation {
    Add,
    Delete
}

impl std::str::FromStr for Operation {
    type Err = ();

    fn from_str(s: &str) -> Result<Operation, ()> {
        match s {
            "ADD" => Ok(Operation::Add),
            "DELETE" => Ok(Operation::Delete),
            _ => Err(()),
        }
    }
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

    pub async fn update_assignees(self, operation: &Operation, hero: String, shift_start_time: i64, assignees: Vec<String>) -> Result<Option<Schedule>, Error> {
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

        let update_expression = match operation {
            Operation::Add => "ADD assignees :a".to_string(),
            Operation::Delete => "DELETE assignees :a".to_string()
        };

        let update_item_input = UpdateItemInput {
            table_name: self.table_name.clone(),
            key: key.clone(),
            update_expression: Some(update_expression),
            expression_attribute_values: Some(expression_attribute_values),
            return_values: Some("ALL_NEW".to_string()),
            ..Default::default()
        };

        let attributes = self.client.update_item(update_item_input).await?.attributes;
        let schedule = Schedule::from_dynamo_item(attributes.expect("Expected attributes from the UpdateItemInput."));

        if schedule.assignees.is_empty() {
            let delete_item_input = DeleteItemInput {
                table_name: self.table_name,
                key,
                ..Default::default()
            };
            self.client.delete_item(delete_item_input).await?;
            Ok(None)
        } else {
            Ok(Some(schedule))
        }
    }
}
