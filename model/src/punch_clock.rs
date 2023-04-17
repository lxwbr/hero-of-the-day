use aws_sdk_dynamodb::model::AttributeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use crate::schedule::Schedule;
use crate::time::days_diff;

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
        let mut previous: i64 = schedules.first().unwrap().shift_start_time;

        schedules.into_iter().for_each(|schedule| {
            schedule.assignees.into_iter().for_each(|member| {
                let days = days_diff(previous, schedule.shift_start_time) as u64;
                if days > 0 {
                    match punch_cards.get(member.as_str()) {
                        None => {
                            punch_cards.insert(member.clone(), PunchClock {
                                hero: hero.clone(),
                                member: member.clone(),
                                days,
                                first_punch: previous,
                                last_punch: previous
                            });
                        }
                        Some(punched) => {
                            let old = punched.clone();
                            punch_cards.insert(member, PunchClock {
                                days: days + old.days.clone(),
                                last_punch: previous,
                                ..old
                            });
                        }
                    }
                }
            });
            previous = schedule.shift_start_time;
        });
    }

    punch_cards.values().cloned().collect()
}
