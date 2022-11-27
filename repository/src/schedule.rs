use maplit::hashmap;
use aws_config::SdkConfig;
use aws_sdk_dynamodb::{Client, model::{AttributeValue, ReturnValue}};
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

    pub async fn update_assignees(
        &self,
        operation: &Operation,
        hero: &String,
        shift_start_time: i64,
        assignees: Vec<String>,
    ) -> Result<Option<Schedule>, Error> {
        let update_expression = match operation {
            Operation::Add => "ADD assignees :a".to_string(),
            Operation::Delete => "DELETE assignees :a".to_string(),
        };

        let update_item_output = self.client
            .update_item()
            .table_name(&self.table_name)
            .key("hero", AttributeValue::S(hero.clone()))
            .key("shift_start_time", AttributeValue::N(shift_start_time.to_string()))
            .update_expression(update_expression)
            .expression_attribute_values(":a", AttributeValue::Ss(assignees))
            .return_values(ReturnValue::AllNew)
            .send()
            .await?;

        let schedule = Schedule::from_dynamo_item(
            update_item_output.attributes().expect("Expected attributes from the UpdateItemInput."),
        );

        if schedule.assignees.is_empty() {
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("hero", AttributeValue::S(hero.clone()))
                .key("shift_start_time", AttributeValue::N(shift_start_time.to_string()))
                .send()
                .await?;
            Ok(None)
        } else {
            Ok(Some(schedule))
        }
    }

    pub async fn get_first_before(
        &self,
        hero: String,
        timestamp: u64,
    ) -> Result<Option<Schedule>, Error> {
        let schedules: Vec<Schedule> = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("hero = :h AND shift_start_time <= :s")
            .expression_attribute_values(":s", AttributeValue::N(timestamp.to_string()))
            .expression_attribute_values(":h", AttributeValue::S(hero))
            .scan_index_forward(false)
            .limit(1)
            .send()
            .await?
            .items
            .unwrap()
            .into_iter()
            .map(|item| Schedule::from_dynamo_item(&item))
            .collect();
        if schedules.is_empty() {
            Ok(None)
        } else {
            Ok(Some(schedules.into_iter().nth(0).unwrap()))
        }
    }
}