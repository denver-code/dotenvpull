use crate::config::get_or_create_config;
use crate::crypto::encrypt;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde_json::json;
use std::fs;

pub async fn update(
    api_url: &str,
    project_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = get_or_create_config()?;
    let project_config = config
        .get(project_name)
        .ok_or("Project not found in config")?;
    let access_key = project_config
        .get("access_key")
        .ok_or("No access key found")?
        .as_str()
        .unwrap();
    let encryption_key = project_config
        .get("encryption_key")
        .ok_or("No encryption key found")?
        .as_str()
        .unwrap();
    let encryption_key_bytes = general_purpose::STANDARD.decode(encryption_key)?;

    let client = Client::new();
    let content = fs::read_to_string(file_path)?;
    let encrypted_content = encrypt(&content, encryption_key_bytes.as_slice().try_into()?);

    let response = client
        .put(&format!("{}/update", api_url))
        .header("X-API-Key", access_key)
        .json(&json!({
            "project_id": project_name,
            "encrypted_content": encrypted_content
        }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("File updated successfully");
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}
