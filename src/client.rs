use std::str;

use actix_web::client::Client;

use serde::Serialize;

const USER_AGENT_KEY: &'static str = "User-Agent";
const UA: &'static str = "PDFiller";

pub async fn get<S: AsRef<str>>(uri: S) -> Option<Vec<u8>> {
    let client_request = Client::default().get(uri.as_ref());
    let response = client_request.header(USER_AGENT_KEY, UA).send().await;

    match response {
        Ok(mut response) => match response.body().await {
            Ok(body) => Some(body.to_vec()),
            _ => None,
        },
        _ => None,
    }
}

#[allow(dead_code)]
pub async fn post<S: AsRef<str>, D: Serialize>(uri: S, request: D) -> Option<Vec<u8>> {
    let client_request = Client::default().post(uri.as_ref());
    let response = client_request
        .header(USER_AGENT_KEY, UA)
        .send_json(&request)
        .await;

    match response {
        Ok(mut response) => match response.body().await {
            Ok(body) => Some(body.to_vec()),
            _ => None,
        },
        _ => None,
    }
}
