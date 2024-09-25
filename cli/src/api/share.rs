use crate::config::get_or_create_config;
use crate::crypto::{decrypt, encrypt};
use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;

pub async fn share(
    api_url: &str,
    project_id: &str,
    include_all_projects: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Project ID: {}", project_id);
    let config = get_or_create_config()?;

    let project_config = if include_all_projects {
        config.clone()
    } else {
        if !config.as_object().unwrap().contains_key(project_id) {
            println!("Error: Project '{}' not found in local config", project_id);
            return Ok(());
        }
        json!({ project_id: config[project_id].clone() })
    };

    let share_code: [u8; 32] = rand::thread_rng().gen();
    let encryption_key: [u8; 32] = rand::thread_rng().gen();

    let share_code_str = general_purpose::STANDARD.encode(&share_code);

    let client = Client::new();
    let response = client
        .post(&format!("{}/share", api_url))
        .json(&json!({
            "project_id": project_id,
            "encrypted_content": encrypt(&project_config.to_string(), &encryption_key),
            "share_code": share_code_str,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("Use this command to share the config:");
        println!(
            "dotenvpull getshared {} {} {} {}",
            share_code_str,
            project_id,
            api_url,
            general_purpose::STANDARD.encode(encryption_key),
        );
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}

pub async fn getshared(
    api_url: &str,
    share_code: &str,
    project_id: &str,
    encryption_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client
        .get(&format!("{}/share", api_url))
        .header("X-Share-Code", share_code)
        .header("X-Project-Id", project_id)
        .send()
        .await?;

    if response.status().is_success() {
        let json: Value = response.json().await?;
        let encrypted_content = json
            .get("encrypted_content")
            .ok_or("No content found")?
            .as_str()
            .unwrap();

        let encryption_key_bytes = general_purpose::STANDARD.decode(encryption_key)?;

        let decrypted_content = decrypt(
            encrypted_content,
            encryption_key_bytes.as_slice().try_into()?,
        );

        let decrypted_config: Value = serde_json::from_str(&decrypted_content)?;

        if decrypted_config.get("api_url").is_some() {
            let _ = fs::remove_file("dotenvpull_config.json");
            fs::write(
                "dotenvpull_config.json",
                serde_json::to_string_pretty(&decrypted_config)?,
            )?;
        } else {
            let mut config = get_or_create_config()?;
            config[project_id] = decrypted_config[project_id].clone();
            fs::write(
                "dotenvpull_config.json",
                serde_json::to_string_pretty(&config)?,
            )?;
        }
        println!(
            "Config for project '{}' shared successfully and added to local config.",
            project_id
        );
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}
