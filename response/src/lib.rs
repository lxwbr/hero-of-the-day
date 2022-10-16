use lambda_http::{Body, http::header::{CONTENT_TYPE, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_ALLOW_CREDENTIALS}, Response};
use serde::Serialize;
use serde_json::{json};

pub fn ok<T>(body: T) -> Response<Body> where T: Serialize {
    Response::builder()
        .status(200)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .body(Body::Text(json!(body).to_string()))
        .expect("failed to render response")
}

pub fn bad_request(body: String) -> Response<Body> {
    Response::builder()
        .status(400)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .body(Body::Text(body))
        .expect("failed to render response")
}

pub fn server_error(body: String) -> Response<Body> {
    Response::builder()
    .status(500)
    .header(CONTENT_TYPE, "application/json")
    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
    .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
    .body(Body::Text(body))
    .expect("failed to render response")
}