use std::str;

use serde::Serialize;

use reqwest::Client;

const USER_AGENT_KEY: &'static str = "User-Agent";
const UA: &'static str = "PDFiller";

pub async fn get<S: AsRef<str>>(uri: S) -> Option<Vec<u8>> {
    let client_request = Client::default().get(uri.as_ref());
    let response = client_request.header(USER_AGENT_KEY, UA).send().await;

    match response {
        Ok(response) => match response.bytes().await {
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
        .json(&request)
        .send()
        .await;

    match response {
        Ok(response) => match response.bytes().await {
            Ok(body) => Some(body.to_vec()),
            _ => None,
        },
        _ => None,
    }
}
