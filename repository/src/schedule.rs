use aws_config::SdkConfig;
use aws_sdk_dynamodb::{
    types::{AttributeValue, ReturnValue},
    Client,
};
use futures::future;
use maplit::hashmap;
use model::schedule::Schedule;
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct ScheduleRepository {
    client: Client,
    table_name: String,
}

#[derive(Debug)]
pub struct LastTwoSchedules {
    pub last: Schedule,
    pub previous_to_last: Option<Schedule>,
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
            client: Client::new(shared_config),
            table_name: env::var("SCHEDULE_TABLE").unwrap(),
        }
    }

    pub fn new_with_table_name(
        shared_config: &SdkConfig,
        table_name: String,
    ) -> ScheduleRepository {
        ScheduleRepository {
            client: Client::new(shared_config),
            table_name: env::var(table_name).unwrap(),
        }
    }

    pub async fn get(
        &self,
        hero: String,
        between: Option<(i64, i64)>,
    ) -> Result<Vec<Schedule>, Error> {
        let mut attribute_values = hashmap! {
            ":hero".to_string() => AttributeValue::S(hero)
        };

        let mut key_condition_expression = "hero = :hero".to_string();

        if let Some((start_time, end_time)) = between {
            attribute_values.insert(":s".to_string(), AttributeValue::N(start_time.to_string()));
            attribute_values.insert(":e".to_string(), AttributeValue::N(end_time.to_string()));
            key_condition_expression = format!(
                "{} AND shift_start_time BETWEEN :s AND :e",
                key_condition_expression
            );
        }

        let mut schedules = vec![];
        let mut exclusive_start_key = None;

        loop {
            let request = self
                .client
                .query()
                .key_condition_expression(key_condition_expression.clone())
                .set_expression_attribute_values(Some(attribute_values.clone()))
                .table_name(&self.table_name)
                .set_exclusive_start_key(exclusive_start_key)
                .send()
                .await?;

            schedules.extend(
                request
                    .items()
                    .iter()
                    .map(Schedule::from_dynamo_item)
                    .collect::<Vec<Schedule>>(),
            );
            match request.last_evaluated_key {
                Some(last_evaluated_key) => {
                    exclusive_start_key = Some(last_evaluated_key.clone());
                }
                None => {
                    break;
                }
            }
        }

        Ok(schedules)
    }

    pub async fn update_assignees(
        &self,
        operation: &Operation,
        hero: &str,
        shift_start_time: i64,
        assignees: Vec<String>,
    ) -> Result<Option<Schedule>, Error> {
        let update_expression = match operation {
            Operation::Add => "ADD assignees :a".to_string(),
            Operation::Delete => "DELETE assignees :a".to_string(),
        };

        let update_item_output = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("hero", AttributeValue::S(hero.to_owned()))
            .key(
                "shift_start_time",
                AttributeValue::N(shift_start_time.to_string()),
            )
            .update_expression(update_expression)
            .expression_attribute_values(":a", AttributeValue::Ss(assignees))
            .return_values(ReturnValue::AllNew)
            .send()
            .await?;

        let schedule = Schedule::from_dynamo_item(
            update_item_output
                .attributes()
                .expect("Expected attributes from the UpdateItemInput."),
        );

        if schedule.assignees.is_empty() {
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("hero", AttributeValue::S(hero.to_owned()))
                .key(
                    "shift_start_time",
                    AttributeValue::N(shift_start_time.to_string()),
                )
                .send()
                .await?;
            Ok(None)
        } else {
            Ok(Some(schedule))
        }
    }

    pub async fn get_last_n_before(
        &self,
        hero: String,
        timestamp: u64,
        n: i32,
    ) -> Result<Vec<Schedule>, Error> {
        let schedules: Vec<Schedule> = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("hero = :h AND shift_start_time <= :s")
            .expression_attribute_values(":s", AttributeValue::N(timestamp.to_string()))
            .expression_attribute_values(":h", AttributeValue::S(hero))
            .scan_index_forward(false)
            .limit(n)
            .send()
            .await?
            .items
            .unwrap()
            .into_iter()
            .map(|item| Schedule::from_dynamo_item(&item))
            .collect();
        if schedules.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(Vec::from_iter(schedules.into_iter()))
        }
    }

    pub async fn get_first_before(
        &self,
        hero: String,
        timestamp: u64,
    ) -> Result<Option<Schedule>, Error> {
        let schedules: Vec<Schedule> = self.get_last_n_before(hero, timestamp, 1).await?;
        if schedules.is_empty() {
            Ok(None)
        } else {
            Ok(Some(schedules.into_iter().nth(0).unwrap()))
        }
    }

    pub async fn get_last_two_before(
        &self,
        hero: String,
        timestamp: u64,
    ) -> Result<Option<LastTwoSchedules>, Error> {
        let schedules: Vec<Schedule> = self.get_last_n_before(hero, timestamp, 2).await?;
        match schedules.len() {
            1 => Ok(Some(LastTwoSchedules {
                last: schedules.into_iter().last().unwrap(),
                previous_to_last: None,
            })),
            2 => {
                let mut iter = schedules.into_iter();
                let last = iter.next().unwrap();
                let previous_to_last = iter.next().unwrap();
                Ok(Some(LastTwoSchedules {
                    last,
                    previous_to_last: Some(previous_to_last),
                }))
            }
            _ => Ok(None),
        }
    }

    pub async fn get_all_repeating_before(
        &self,
        hero: String,
        timestamp: u64,
    ) -> Result<Vec<Schedule>, Error> {
        let schedules: Vec<Schedule> = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("hero = :h AND shift_start_time <= :s")
            .expression_attribute_values(":s", AttributeValue::N(timestamp.to_string()))
            .expression_attribute_values(":h", AttributeValue::S(hero))
            .filter_expression("attribute_exists(repeat_every_days)")
            .send()
            .await?
            .items
            .unwrap()
            .into_iter()
            .map(|item| Schedule::from_dynamo_item(&item))
            .collect();
        Ok(schedules)
    }

    pub async fn list(&self) -> Result<Vec<Schedule>, Error> {
        let response = self
            .client
            .scan()
            .table_name(&self.table_name)
            .send()
            .await?;
        let heroes: Vec<Schedule> = response
            .items()
            .iter()
            .map(Schedule::from_dynamo_item)
            .collect();
        Ok(heroes)
    }

    pub async fn put(&self, schedule: &Schedule) -> Result<(), Error> {
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("hero", AttributeValue::S(schedule.hero.to_string()))
            .item(
                "shift_start_time",
                AttributeValue::N(schedule.shift_start_time.to_string()),
            )
            .item("assignees", AttributeValue::Ss(schedule.assignees.clone()))
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete(&self, hero_name: String) -> Result<(), Error> {
        let schedules = self.get(hero_name, None).await?;

        let _ = future::try_join_all(schedules.iter().map(|schedule| {
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("hero", AttributeValue::S(schedule.hero.to_string()))
                .key(
                    "shift_start_time",
                    AttributeValue::N(schedule.shift_start_time.to_string()),
                )
                .send()
        }))
        .await?;
        Ok(())
    }
}
