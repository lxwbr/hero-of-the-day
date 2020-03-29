use lambda::lambda;
use serde_json::Value;
use std::env;

extern crate rusoto_core;
extern crate rusoto_dynamodb;
 
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, ScanInput};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[lambda]
#[tokio::main]
async fn main(event: Value) -> Result<Value, Error> {
    let client = DynamoDbClient::new(Region::default());
    let heroes_table_env = "HEROES_TABLE";

    let table_name = env::var(heroes_table_env).expect("Expected environment variable HEROES_TABLE not set");
    
    let scan_input: ScanInput = ScanInput {
        table_name: String::from(table_name),
        ..Default::default()
    };

    match client.scan(scan_input).await {
        Ok(output) => {
            match output.items {
                Some(items) => {
                    println!("Items in table:");

                    for item in items {
                        println!("Hero: {:?}, Members: ${:?}", item["name"].s, item["members"].ss);
                    }
                },
                None => println!("No items in table!"),
            }
            Ok(event)
        },
        Err(error) => {
            println!("Error: {:?}", error);
            Err(error.into())
        },
    }
}
