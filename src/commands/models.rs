use crate::api;
use anyhow::Result;
use serde_json::Value;
use std::cmp::Ordering;

pub fn run(model_type: &str, verbose: bool) -> Result<()> {
    let client = api::http(30)?;
    let mut models = api::fetch_models(&client, model_type)?;
    if model_type == "image" {
        models.sort_by(|a, b| {
            price_generation(b)
                .partial_cmp(&price_generation(a))
                .unwrap_or(Ordering::Equal)
        });
    }

    for model in &models {
        let id = model["id"].as_str().unwrap_or("?");
        let spec = &model["model_spec"];
        let price = price_generation(model);
        let caps = caps_string(spec);
        let price_str = if price > 0.0 {
            format!("${price:.6}/req")
        } else {
            String::new()
        };
        let mut line = format!("{id:<40} {price_str:<20} {caps}");
        if verbose {
            if let Some(description) = spec["description"].as_str() {
                if !description.is_empty() {
                    line.push_str(&format!(
                        "  -- {}",
                        crate::util::truncate(description, 80)
                    ));
                }
            }
        }
        println!("{line}");
    }
    Ok(())
}

fn price_generation(model: &Value) -> f64 {
    model["model_spec"]["pricing"]["generation"]["usd"]
        .as_f64()
        .unwrap_or(0.0)
}

fn caps_string(spec: &Value) -> String {
    match &spec["capabilities"] {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(map) => map
            .iter()
            .filter(|(_, value)| value.as_bool() == Some(true))
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}
