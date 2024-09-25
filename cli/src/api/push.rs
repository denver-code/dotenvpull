use crate::config::update_config;
use crate::crypto::encrypt;
use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use reqwest::Client;
use serde_json::json;
use std::fs;

pub async fn push(
    api_url: &str,
    project_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let encryption_key: [u8; 32] = rand::thread_rng().gen();
    let encrypted_content = encrypt(&content, &encryption_key);

    println!("{}", content);
    println!("{}", api_url);

    let client = Client::new();
    let response = client
        .post(&format!("{}/push", api_url))
        .json(&json!({
            "project_id": project_name,
            "encrypted_content": encrypted_content
        }))
        .send()
        .await?;

    println!("{}", response.status());

    if response.status().is_success() {
        let json = response.json::<serde_json::Value>().await?;
        let access_key = json["access_key"].as_str().unwrap();
        update_config(
            project_name,
            access_key,
            &general_purpose::STANDARD.encode(encryption_key),
        )?;
        println!("File pushed successfully");
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}
