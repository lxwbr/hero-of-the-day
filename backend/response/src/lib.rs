use serde_json::{ Value, json };

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

pub fn server_error(body: String) -> Value {
    json!({
        "statusCode": 500,
        "headers": headers(),
        "body": body
    })
}
