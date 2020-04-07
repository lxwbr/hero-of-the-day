use serde_json::{json, Value};

fn headers() -> Value {
    json!({
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Credentials": true
    })
}

pub fn ok(body: String) -> Value {
    json!({
        "statusCode": 200,
        "headers": headers(),
        "body": body
    })
}

pub fn bad_request(body: String) -> Value {
    json!({
        "statusCode": 400,
        "headers": headers(),
        "body": body
    })
}

pub fn server_error(body: String) -> Value {
    json!({
        "statusCode": 500,
        "headers": headers(),
        "body": body
    })
}
