use crate::config::get_or_create_config;
use crate::crypto::decrypt;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn pull(
    api_url: &str,
    project_name: &str,
    output_file: &str,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if Path::new(output_file).exists() && !force {
        println!("Error: Output file already exists. Use --force to overwrite.");
        return Ok(());
    }

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
    let response = client
        .get(&format!("{}/pull", api_url))
        .header("X-API-Key", access_key)
        .send()
        .await?;

    if response.status().is_success() {
        let json: Value = response.json().await?;
        let encrypted_content = json
            .get("encrypted_content")
            .ok_or("No content found")?
            .as_str()
            .unwrap();
        let decrypted_content = decrypt(
            encrypted_content,
            encryption_key_bytes.as_slice().try_into()?,
        );
        fs::write(output_file, decrypted_content)?;
        println!("File pulled successfully and saved to {}", output_file);
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}
