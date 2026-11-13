use crate::api;
use crate::util::{open_file, timestamp};
use anyhow::{Context, Result};
use base64::Engine;
use serde_json::json;
use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::Duration;

pub struct Args {
    pub prompt: String,
    pub model: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub out: Option<PathBuf>,
    pub no_open: bool,
}

pub fn run(args: Args) -> Result<()> {
    let client = api::http(30)?;
    let model = match args.model {
        Some(model) => model,
        None => {
            let models = api::fetch_models(&client, "image")?;
            let best = models
                .iter()
                .max_by(|a, b| {
                    price_generation(a)
                        .partial_cmp(&price_generation(b))
                        .unwrap_or(Ordering::Equal)
                })
                .context("no image models available")?;
            let id = best["id"].as_str().unwrap_or("?").to_string();
            eprintln!("Using model: {id}");
            id
        }
    };

    let mut payload = json!({
        "prompt": args.prompt,
        "format": "png",
        "return_binary": true,
        "safe_mode": false,
        "hide_watermark": true,
        "model": model,
    });

    if ["gpt-image-2", "nano-banana-2", "nano-banana-pro"].contains(&model.as_str()) {
        if let Some(value) = args.aspect_ratio {
            payload["aspect_ratio"] = json!(value);
        }
        if let Some(value) = args.resolution {
            payload["resolution"] = json!(value);
        }
    } else {
        if let Some(value) = args.width {
            payload["width"] = json!(value);
        }
        if let Some(value) = args.height {
            payload["height"] = json!(value);
        }
    }

    eprintln!("Generating image...");
    let response = client
        .post(api::url("/image/generate"))
        .headers(api::headers()?)
        .json(&payload)
        .timeout(Duration::from_secs(300))
        .send()?;
    let response = api::ensure_success(response)?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = if content_type.starts_with("image/") {
        response.bytes()?.to_vec()
    } else {
        let data: serde_json::Value = response.json()?;
        let encoded = data["images"][0]
            .as_str()
            .context("no image in response")?;
        base64::engine::general_purpose::STANDARD.decode(encoded)?
    };

    let path = args
        .out
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/venice_image_{}.png", timestamp())));
    std::fs::write(&path, &bytes)?;
    println!("Saved: {}", path.display());

    if !args.no_open {
        open_file(&path);
    }
    Ok(())
}

fn price_generation(model: &serde_json::Value) -> f64 {
    model["model_spec"]["pricing"]["generation"]["usd"]
        .as_f64()
        .unwrap_or(0.0)
}
