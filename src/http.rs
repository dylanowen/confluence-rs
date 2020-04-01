//! HTTP helpers.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
pub use reqwest::Error as HttpError;
pub use reqwest::StatusCode;
use std::result;

/// Simplified HTTP response representation.
#[derive(Debug)]
pub struct Response {
    pub status: StatusCode,
    pub body: String,
}

/// Perform a GET request to specified URL.
pub async fn get(url: &str) -> Result<Response> {
    let response = reqwest::get(url).await?;
    let status = response.status();
    let body = response.text().await?;

    Ok(Response { status, body })
}

/// Perform a SOAP action to specified URL.
pub async fn soap_action(url: &str, action: &str, xml: &str, client: &Client) -> Result<Response> {
    let soap_action = HeaderName::from_bytes(b"SOAPAction").unwrap();
    let soap_value = HeaderValue::from_str(action).unwrap();
    let mut hmap = HeaderMap::new();
    hmap.insert(CONTENT_TYPE, "text/xml; charset=utf-8".parse().unwrap());
    hmap.insert(soap_action, soap_value);

    //let client = reqwest::Client::new();
    let response = client
        .post(url)
        .headers(hmap)
        .body(xml.to_string())
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    Ok(Response { status, body })
}

pub type Result<T> = result::Result<T, HttpError>;
