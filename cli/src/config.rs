use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub fn get_or_create_config() -> Result<Value, Box<dyn std::error::Error>> {
    let config_path = Path::new("dotenvpull_config.json");
    if config_path.exists() {
        let config_str = fs::read_to_string(config_path)?;
        Ok(serde_json::from_str(&config_str)?)
    } else {
        let config = json!({
            "api_url": "http://localhost:8080"
        });
        fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(config)
    }
}

pub fn update_config(
    project_name: &str,
    access_key: &str,
    encryption_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = get_or_create_config()?;
    config[project_name] = json!({
        "access_key": access_key,
        "encryption_key": encryption_key
    });
    fs::write(
        "dotenvpull_config.json",
        serde_json::to_string_pretty(&config)?,
    )?;
    Ok(())
}
