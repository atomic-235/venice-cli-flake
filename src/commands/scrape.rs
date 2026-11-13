use crate::api;
use anyhow::Result;
use serde_json::json;
use std::path::Path;

pub fn run(page_url: &str, format: Option<&str>, out: Option<&Path>, json_out: bool) -> Result<()> {
    let mut payload = json!({"url": page_url});
    if let Some(format) = format {
        payload["format"] = json!(format);
    }

    let client = api::http(60)?;
    let data = api::post_json(&client, "/augment/scrape", &payload)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    let text = data["markdown"]
        .as_str()
        .or_else(|| data["content"].as_str())
        .or_else(|| data["text"].as_str());
    match (text, out) {
        (Some(text), Some(path)) => {
            std::fs::write(path, text)?;
            println!("Saved: {}", path.display());
        }
        (Some(text), None) => println!("{text}"),
        (None, _) => println!("{}", serde_json::to_string_pretty(&data)?),
    }
    Ok(())
}
