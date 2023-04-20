use crate::schedule::Schedule;
use crate::time::days_diff;
use aws_sdk_dynamodb::model::AttributeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PunchClock {
    pub hero: String,
    pub member: String,
    pub days: u64,
    pub first_punch: i64,
    pub last_punch: i64,
}

impl PunchClock {
    pub fn from_dynamo_item(item: &HashMap<String, AttributeValue>) -> PunchClock {
        PunchClock {
            hero: item["hero"]
                .as_s()
                .expect("Hero name is missing")
                .to_owned(),
            member: item["member"]
                .as_s()
                .expect("Member name is missing")
                .to_owned(),
            days: u64::from_str(item["days"].as_n().unwrap_or(&"0".to_string()))
                .expect("Days was not a number")
                .to_owned(),
            first_punch: i64::from_str(item["first_punch"].as_n().unwrap_or(&"0".to_string()))
                .expect("First punch was not a number")
                .to_owned(),
            last_punch: i64::from_str(item["last_punch"].as_n().unwrap_or(&"0".to_string()))
                .expect("Last punch was not a number")
                .to_owned(),
        }
    }
}

pub fn recalculate_punch_time(hero: String, schedules: Vec<Schedule>) -> Vec<PunchClock> {
    let mut punch_cards: HashMap<String, PunchClock> = HashMap::new();

    if !schedules.is_empty() {
        let mut previous: Schedule = schedules.first().unwrap().clone();

        schedules.into_iter().skip(1).for_each(|schedule| {
            let days = days_diff(previous.shift_start_time, schedule.shift_start_time) as u64;
            if previous
                .assignees
                .contains(&"marcel.mindemann@moia.io".to_string())
            {
                println!("previous: {:?}, current: {:?}", previous, schedule);
            }
            if days > 0 {
                previous.assignees.clone().into_iter().for_each(|assignee| {
                    match punch_cards.get(assignee.as_str()) {
                        None => {
                            let punch_card = PunchClock {
                                hero: hero.clone(),
                                member: assignee.clone(),
                                days,
                                first_punch: previous.shift_start_time,
                                last_punch: previous.shift_start_time,
                            };
                            if assignee == "marcel.mindemann@moia.io" {
                                println!("{:?}", punch_card);
                            }
                            punch_cards.insert(assignee.clone(), punch_card);
                        }
                        Some(punched) => {
                            let old = punched.clone();
                            let punch_card = PunchClock {
                                days: days + old.days.clone(),
                                last_punch: previous.shift_start_time,
                                ..old
                            };
                            if assignee == "marcel.mindemann@moia.io" {
                                println!("Some {:?}", punch_card);
                            }
                            punch_cards.insert(assignee.clone(), punch_card);
                        }
                    }
                });
            }
            previous = schedule.clone();
        });
    }

    punch_cards.values().cloned().collect()
}
