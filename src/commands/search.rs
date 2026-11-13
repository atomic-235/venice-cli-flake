use crate::api;
use crate::util::{plain, truncate};
use anyhow::{bail, Result};
use serde_json::json;

pub fn run(query: &str, provider: Option<&str>, limit: u8, json_out: bool) -> Result<()> {
    if !(1..=20).contains(&limit) {
        bail!("--limit must be between 1 and 20");
    }

    let mut payload = json!({"query": query, "limit": limit});
    if let Some(provider) = provider {
        payload["search_provider"] = json!(provider);
    }

    let client = api::http(60)?;
    let data = api::post_json(&client, "/augment/search", &payload)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    let results = data["results"].as_array().cloned().unwrap_or_default();
    if results.is_empty() {
        println!("No results.");
        return Ok(());
    }

    for (index, result) in results.iter().enumerate() {
        let title = plain(result["title"].as_str().unwrap_or(""));
        let date = result["date"]
            .as_str()
            .filter(|date| !date.is_empty())
            .map(|date| format!("  ({})", plain(date)))
            .unwrap_or_default();
        println!("{}. {title}{date}", index + 1);
        println!("   {}", result["url"].as_str().unwrap_or(""));

        let snippet = plain(result["content"].as_str().unwrap_or(""))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !snippet.is_empty() {
            let suffix = if snippet.chars().count() > 300 {
                "..."
            } else {
                ""
            };
            println!("   {}{suffix}", truncate(&snippet, 300));
        }
    }
    Ok(())
}
