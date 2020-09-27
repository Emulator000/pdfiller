use std::str;

use actix_web::client::{Client, ClientRequest};

use serde::Serialize;

const USER_AGENT_KEY: &'static str = "User-Agent";
const UA: &'static str = "PDFiller";

fn get_client(mut request: ClientRequest) -> ClientRequest {
    request
}

pub async fn get<S: AsRef<str>>(uri: S) -> Option<Vec<u8>> {
    let response = get_client(Client::default().get(uri.as_ref()))
        .header(USER_AGENT_KEY, UA)
        .send()
        .await;

    match response {
        Ok(mut response) => match response.body().await {
            Ok(body) => Some(body.to_vec()),
            _ => None,
        },
        _ => None,
    }
}

pub async fn post<S: AsRef<str>, D: Serialize>(uri: S, request: D) -> Option<Vec<u8>> {
    let response = get_client(Client::default().post(uri.as_ref()))
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
