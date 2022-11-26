use maplit::hashmap;
use aws_config::SdkConfig;
use aws_sdk_dynamodb::{Client, model::{AttributeValue}};
use model::schedule::Schedule;
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct ScheduleRepository {
    client: Client,
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

impl ScheduleRepository {
    pub fn new(shared_config: &SdkConfig) -> ScheduleRepository {
        ScheduleRepository {
            client: Client::new(&shared_config),
            table_name: env::var("SCHEDULE_TABLE").unwrap(),
        }
    }

    pub fn new_with_table_name(shared_config: &SdkConfig, table_name: String) -> ScheduleRepository {
        ScheduleRepository {
            client: Client::new(&shared_config),
            table_name: env::var(table_name).unwrap()
        }
    }

    pub async fn get(&self, hero: String, shift_start_time: Option<i64>) -> Result<Vec<Schedule>, Error> {
        let mut attribute_values = hashmap! {
            ":hero".to_string() => AttributeValue::S(hero)
        };

        let mut key_condition_expression = "hero = :hero".to_string();

        if let Some(time) = shift_start_time {
            attribute_values.insert(
                ":shift_start_time".to_string(), AttributeValue::N(time.to_string())
            );
            key_condition_expression = format!("{} AND shift_start_time = :shift_start_time", key_condition_expression);
        }

        let response = self.client
            .query()
            .key_condition_expression(key_condition_expression)
            .set_expression_attribute_values(Some(attribute_values))
            .table_name(&self.table_name)
            .send()
            .await?;

        let schedules: Vec<Schedule> = response
            .items()
            .unwrap_or_default()
            .into_iter()
            .map(Schedule::from_dynamo_item)
            .collect();
        Ok(schedules)
    }
}