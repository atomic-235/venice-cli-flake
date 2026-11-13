use crate::api;
use crate::commands::{capability, print_model_line, stream_chat};
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub fn run(
    video: Option<String>,
    prompt: Option<String>,
    model: Option<String>,
    list_models: bool,
    verbose: bool,
) -> Result<()> {
    let client = api::http(30)?;

    if list_models {
        for model in api::fetch_models(&client, "text")?
            .iter()
            .filter(|model| capability(model, "supportsVideoInput"))
        {
            print_model_line(model, verbose);
        }
        return Ok(());
    }

    let video_url = video.context("video URL required (or use --list-models)")?;
    let prompt = prompt.unwrap_or_else(|| "Transcribe this video.".into());

    let model = match model {
        Some(model) => model,
        None => {
            let models = api::fetch_models(&client, "text")?;
            let video_models = models
                .iter()
                .filter(|model| capability(model, "supportsVideoInput"))
                .cloned()
                .collect::<Vec<_>>();
            let id = pick_video_model(&video_models)
                .context("no video-capable models found, specify --model")?;
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
                {"type": "video_url", "video_url": {"url": video_url}}
            ]
        }]
    });

    eprintln!("Analyzing video...");
    stream_chat(&client, &payload, 300)
}

fn flash_version(id: &str) -> Option<(u32, Option<u32>)> {
    let rest = id.strip_prefix("gemini-")?.strip_suffix("-flash")?;
    let mut parts = rest.split('-');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().and_then(|part| part.parse::<u32>().ok());
    Some((major, minor))
}

fn pick_video_model(models: &[Value]) -> Option<String> {
    let mut best: Option<(String, (u32, Option<u32>))> = None;
    for model in models {
        if let Some(id) = model["id"].as_str() {
            if let Some(version) = flash_version(id) {
                if best
                    .as_ref()
                    .is_none_or(|(_, current)| version > *current)
                {
                    best = Some((id.to_string(), version));
                }
            }
        }
    }
    best.map(|(id, _)| id).or_else(|| {
        models
            .first()
            .and_then(|model| model["id"].as_str().map(str::to_string))
    })
}
