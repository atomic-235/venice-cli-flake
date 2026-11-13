use crate::api;
use anyhow::Result;
use serde_json::json;

pub fn run(query: &str, model: &str, json_out: bool) -> Result<()> {
    let payload = json!({
        "model": model,
        "messages": [{"role": "user", "content": query}],
        "venice_parameters": {
            "enable_x_search": true,
            "include_venice_system_prompt": false,
        },
    });

    let client = api::http(120)?;
    let data = api::post_json(&client, "/chat/completions", &payload)?;
    if json_out {
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    match data["choices"][0]["message"]["content"].as_str() {
        Some(content) => println!("{content}"),
        None => println!("{}", serde_json::to_string_pretty(&data)?),
    }
    Ok(())
}
