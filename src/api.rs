use anyhow::{bail, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use std::time::Duration;

pub const API_BASE: &str = "https://api.venice.ai/api/v1";

pub fn api_key() -> Result<String> {
    std::env::var("VENICE_API_KEY").context("VENICE_API_KEY environment variable not set")
}

fn proxy_url() -> Option<String> {
    for var in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(var) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    let port = std::env::var("PROXY_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(20172);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(300))
        .ok()
        .map(|_| format!("http://127.0.0.1:{port}"))
}

pub fn http(timeout_secs: u64) -> Result<Client> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .no_proxy();
    if let Some(proxy) = proxy_url() {
        builder = builder.proxy(reqwest::Proxy::all(&proxy)?);
    }
    builder.build().context("build HTTP client")
}

pub fn headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {}", api_key()?);
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&value)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

pub fn url(path: &str) -> String {
    format!("{API_BASE}{path}")
}

pub fn ensure_success(response: Response) -> Result<Response> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("Venice API {status}: {body}");
    }
    Ok(response)
}

pub fn get_json(client: &Client, path: &str, query: &[(&str, &str)]) -> Result<Value> {
    let response = client
        .get(url(path))
        .query(query)
        .headers(headers()?)
        .send()?;
    ensure_success(response)?.json().context("decode JSON response")
}

pub fn post_json(client: &Client, path: &str, body: &Value) -> Result<Value> {
    let response = client
        .post(url(path))
        .headers(headers()?)
        .json(body)
        .send()?;
    ensure_success(response)?.json().context("decode JSON response")
}

pub fn fetch_models(client: &Client, model_type: &str) -> Result<Vec<Value>> {
    let data = get_json(client, "/models", &[("type", model_type)])?;
    Ok(data
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}
