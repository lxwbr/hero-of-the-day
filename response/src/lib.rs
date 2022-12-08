use lambda_http::{Body, Error, Response, http::header::{CONTENT_TYPE, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_ALLOW_CREDENTIALS}};
use serde::Serialize;
use serde_json::{json};

pub fn ok<T>(body: T) -> Result<Response<Body>, Error> where T: Serialize {
    Ok::<Response<Body>, Error>(Response::builder()
        .status(200)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .body(Body::Text(json!(body).to_string()))
        .expect("failed to render response"))
}

pub fn bad_request(body: String) -> Result<Response<Body>, Error> {
    Ok::<Response<Body>, Error>(Response::builder()
        .status(400)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .body(Body::Text(body))
        .expect("failed to render response"))
}

pub fn server_error(body: String) -> Result<Response<Body>, Error> {
    Ok::<Response<Body>, Error>(Response::builder()
    .status(500)
    .header(CONTENT_TYPE, "application/json")
    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
    .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
    .body(Body::Text(body))
    .expect("failed to render response"))
}