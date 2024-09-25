use crate::config::get_or_create_config;
use reqwest::Client;
use std::fs;

pub async fn delete(api_url: &str, project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = get_or_create_config()?;
    let project_config = config
        .get(project_name)
        .ok_or("Project not found in config")?;
    let access_key = project_config
        .get("access_key")
        .ok_or("No access key found")?
        .as_str()
        .unwrap();

    let client = Client::new();
    let response = client
        .delete(&format!("{}/delete", api_url))
        .header("X-API-Key", access_key)
        .send()
        .await?;

    if response.status().is_success() {
        println!("File deleted successfully");
        config.as_object_mut().unwrap().remove(project_name);
        fs::write(
            "dotenvpull_config.json",
            serde_json::to_string_pretty(&config)?,
        )?;
        println!("Project '{}' removed from local config", project_name);
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}
