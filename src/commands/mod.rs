pub mod balance;
pub mod image;
pub mod models;
pub mod scrape;
pub mod search;
pub mod tts;
pub mod video;
pub mod vision;
pub mod xsearch;

use crate::api;
use crate::util::truncate;
use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

pub(crate) fn print_model_line(model: &Value, verbose: bool) {
    let id = model["id"].as_str().unwrap_or("?");
    let mut line = format!("{id:<40}");
    if verbose {
        if let Some(description) = model["model_spec"]["description"].as_str() {
            if !description.is_empty() {
                line.push_str(&format!("  -- {}", truncate(description, 80)));
            }
        }
    }
    println!("{line}");
}

pub(crate) fn capability(model: &Value, key: &str) -> bool {
    model["model_spec"]["capabilities"][key].as_bool() == Some(true)
}

pub(crate) fn stream_chat(client: &Client, payload: &Value, timeout_secs: u64) -> Result<()> {
    let response = client
        .post(api::url("/chat/completions"))
        .headers(api::headers()?)
        .json(payload)
        .timeout(Duration::from_secs(timeout_secs))
        .send()?;
    let response = api::ensure_success(response)?;

    let mut reader = BufReader::new(response);
    let mut line = String::new();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            if let Ok(chunk) = serde_json::from_str::<Value>(data) {
                if let Some(delta) = chunk["choices"][0]["delta"]["content"].as_str() {
                    write!(out, "{delta}")?;
                    out.flush()?;
                }
            }
        }
    }
    writeln!(out)?;
    Ok(())
}
