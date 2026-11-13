use crate::api;
use anyhow::Result;
use serde_json::Value;

pub fn run() -> Result<()> {
    let client = api::http(30)?;
    let data = api::get_json(&client, "/api_keys/rate_limits", &[])?;
    let data = data.get("data").cloned().unwrap_or(Value::Null);

    let tier = &data["apiTier"];
    let balances = &data["balances"];
    let usd = balances["USD"].as_f64().unwrap_or(0.0);
    let diem = balances["DIEM"].as_f64().unwrap_or(0.0);
    let tier_id = tier["id"].as_str().unwrap_or("?");
    let charged = tier["isCharged"].as_bool().unwrap_or(false);
    let access = data["accessPermitted"].as_bool().unwrap_or(false);

    println!(
        "Tier:      {tier_id} ({})",
        if charged { "charged" } else { "free" }
    );
    println!(
        "Access:    {}",
        if access { "permitted" } else { "denied" }
    );
    println!("USD:       ${usd:.2}");
    println!("DIEM:      {diem:.2}");
    if let Some(expiration) = data["keyExpiration"].as_str() {
        println!("Expires:   {expiration}");
    }
    Ok(())
}
