use maplit::hashmap;
use model::schedule::Schedule;
use rusoto_dynamodb::{
    AttributeValue, DeleteItemInput, DynamoDb, DynamoDbClient, QueryInput, UpdateItemInput,
};
use std::env;

#[path = "error.rs"]
mod error;
use error::RepositoryError;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct ScheduleRepository<'a> {
    client: &'a DynamoDbClient,
    table_name: String,
}

pub enum Operation {
    Add,
    Delete,
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

impl ScheduleRepository<'_> {
    pub fn new(client: &DynamoDbClient) -> ScheduleRepository {
        ScheduleRepository {
            client,
            table_name: env::var("SCHEDULE_TABLE").unwrap(),
        }
    }

    pub async fn get(self, hero: String, shift_start_time: Option<i64>) -> Result<Vec<Schedule>, Error> {
        let mut attribute_values = hashmap! {
            ":hero".to_owned() => AttributeValue {
                s: Some(hero),
                ..Default::default()
            }
        };

        let mut key_condition_expression = "hero = :hero".to_string();

        if let Some(time) = shift_start_time {
            attribute_values.insert(
                ":shift_start_time".to_owned(), AttributeValue {
                    n: Some(time.to_string()),
                    ..Default::default()
                }
            );
            key_condition_expression = format!("{} AND shift_start_time = :shift_start_time", key_condition_expression);
        }

        let query_input = QueryInput {
            table_name: self.table_name,
            key_condition_expression: Some(key_condition_expression),
            expression_attribute_values: Some(attribute_values),
            ..Default::default()
        };

        let schedules: Vec<Schedule> = self
            .client
            .query(query_input)
            .await?
            .items
            .ok_or(RepositoryError::NoneScan)?
            .into_iter()
            .map(Schedule::from_dynamo_item)
            .collect();

        Ok(schedules)
    }

    pub async fn update_assignees(
        self,
        operation: &Operation,
        hero: String,
        shift_start_time: i64,
        assignees: Vec<String>,
    ) -> Result<Option<Schedule>, Error> {
        let key = hashmap! {
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
            Operation::Delete => "DELETE assignees :a".to_string(),
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
        let schedule = Schedule::from_dynamo_item(
            attributes.expect("Expected attributes from the UpdateItemInput."),
        );

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

    pub async fn get_first_before(
        self,
        hero: String,
        timestamp: u64,
    ) -> Result<Option<Schedule>, Error> {
        let expression_attribute_values = hashmap! {
            ":s".to_string() => AttributeValue {
                n: Some(timestamp.to_string()),
                ..Default::default()
            },
            ":h".to_string() => AttributeValue {
                s: Some(hero),
                ..Default::default()
            }
        };

        let query_input = QueryInput {
            table_name: self.table_name,
            key_condition_expression: Some("hero = :h AND shift_start_time <= :s".to_string()),
            expression_attribute_values: Some(expression_attribute_values),
            scan_index_forward: Some(false),
            limit: Some(1),
            ..Default::default()
        };

        let schedules: Vec<Schedule> = self
            .client
            .query(query_input)
            .await?
            .items
            .unwrap()
            .into_iter()
            .map(Schedule::from_dynamo_item)
            .collect();
        if schedules.is_empty() {
            Ok(None)
        } else {
            Ok(Some(schedules.into_iter().nth(0).unwrap()))
        }
    }
}
