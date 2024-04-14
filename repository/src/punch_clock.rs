use aws_config::SdkConfig;
use aws_sdk_dynamodb::Client;
use maplit::hashmap;
use model::punch_clock::PunchClock;
use std::env;
use aws_sdk_dynamodb::types::AttributeValue;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct PunchClockRepository {
    client: Client,
    table_name: String,
}

impl PunchClockRepository {
    pub fn new(shared_config: &SdkConfig) -> PunchClockRepository {
        PunchClockRepository {
            client: Client::new(&shared_config),
            table_name: env::var("PUNCH_CLOCK_TABLE").unwrap(),
        }
    }

    pub fn new_with_table_name(
        shared_config: &SdkConfig,
        table_name: String,
    ) -> PunchClockRepository {
        PunchClockRepository {
            client: Client::new(&shared_config),
            table_name: env::var(table_name).unwrap(),
        }
    }

    pub async fn get(&self, hero: &String, member: String) -> Result<Option<PunchClock>, Error> {
        let punch_clock = self
            .client
            .get_item()
            .key("hero", AttributeValue::S(hero.clone()))
            .key("member", AttributeValue::S(member.clone()))
            .table_name(&self.table_name)
            .send()
            .await?
            .item()
            .map(PunchClock::from_dynamo_item);

        Ok(punch_clock)
    }

    pub async fn get_all(&self, hero: String) -> Result<Vec<PunchClock>, Error> {
        let attribute_values = hashmap! {
            ":hero".to_string() => AttributeValue::S(hero)
        };

        let key_condition_expression = "hero = :hero".to_string();

        let punch_cards = self
            .client
            .query()
            .key_condition_expression(key_condition_expression)
            .set_expression_attribute_values(Some(attribute_values))
            .table_name(&self.table_name)
            .send()
            .await?
            .items()
            .into_iter()
            .map(PunchClock::from_dynamo_item)
            .collect();

        Ok(punch_cards)
    }

    pub async fn put(&self, punch_clock: &PunchClock) -> Result<(), Error> {
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("hero", AttributeValue::S(punch_clock.hero.to_string()))
            .item("member", AttributeValue::S(punch_clock.member.to_string()))
            .item(
                "days",
                AttributeValue::N(punch_clock.days.to_string().clone()),
            )
            .item(
                "last_punch",
                AttributeValue::N(punch_clock.last_punch.to_string().clone()),
            )
            .item(
                "first_punch",
                AttributeValue::N(punch_clock.first_punch.to_string().clone()),
            )
            .send()
            .await?;
        Ok(())
    }
}
