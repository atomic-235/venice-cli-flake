use crate::api;
use crate::commands::{capability, print_model_line, stream_chat};
use crate::util::mime_for;
use anyhow::{Context, Result};
use base64::Engine;
use serde_json::json;
use std::path::PathBuf;

pub fn run(
    image: Option<PathBuf>,
    prompt: Option<String>,
    model: Option<String>,
    list_models: bool,
    verbose: bool,
) -> Result<()> {
    let client = api::http(30)?;

    if list_models {
        for model in api::fetch_models(&client, "text")?
            .iter()
            .filter(|model| capability(model, "supportsVision"))
        {
            print_model_line(model, verbose);
        }
        return Ok(());
    }

    let path = image.context("image path required (or use --list-models)")?;
    let bytes = std::fs::read(&path)
        .with_context(|| format!("image not found: {}", path.display()))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let prompt = prompt.unwrap_or_else(|| "Describe this image in detail.".into());

    let model = match model {
        Some(model) => model,
        None => {
            let models = api::fetch_models(&client, "text")?;
            let best = models
                .iter()
                .find(|model| capability(model, "supportsVision"))
                .context("no vision-capable models found, specify --model")?;
            let id = best["id"].as_str().unwrap_or("?").to_string();
            eprintln!("Using model: {id}");
            id
        }
    };

    let payload = json!({
        "model": model,
        "stream": true,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {
                    "url": format!("data:{};base64,{encoded}", mime_for(&path))
                }}
            ]
        }]
    });

    eprintln!("Analyzing image...");
    stream_chat(&client, &payload, 120)
}
