use crate::api;
use crate::util::{play_audio, timestamp};
use anyhow::{bail, Result};
use serde_json::json;
use std::path::PathBuf;

pub struct Args {
    pub text: String,
    pub voice: String,
    pub model: Option<String>,
    pub format: String,
    pub speed: Option<f64>,
    pub out: Option<PathBuf>,
    pub no_play: bool,
}

pub fn run(args: Args) -> Result<()> {
    let mut payload = json!({
        "model": args.model.unwrap_or_else(|| "tts-minimax-speech-02-hd".into()),
        "input": args.text,
        "voice": args.voice,
        "response_format": args.format,
    });
    if let Some(speed) = args.speed {
        payload["speed"] = json!(speed);
    }

    eprintln!("Generating speech...");
    let client = api::http(120)?;
    let response = client
        .post(api::url("/audio/speech"))
        .headers(api::headers()?)
        .json(&payload)
        .send()?;
    let response = api::ensure_success(response)?;
    let bytes = response.bytes()?.to_vec();
    if bytes.len() < 100 {
        bail!(
            "response too small ({} bytes), possible API error",
            bytes.len()
        );
    }

    let path = args.out.unwrap_or_else(|| {
        PathBuf::from(format!("/tmp/venice_tts_{}.{}", timestamp(), args.format))
    });
    std::fs::write(&path, &bytes)?;
    println!(
        "Saved: {} ({:.1} KB)",
        path.display(),
        bytes.len() as f64 / 1024.0
    );

    if !args.no_play {
        play_audio(&path);
    }
    Ok(())
}
