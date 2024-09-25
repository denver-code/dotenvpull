use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use clap::{App, Arg, SubCommand};
use rand::Rng;
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("DotEnvPull")
        .version("1.0")
        .author("Ihor Savenko (@denver-code)")
        .about("Manages .env files")
        .subcommand(
            SubCommand::with_name("push")
                .about("Push a .env or config file to the server")
                .arg(Arg::with_name("project_name").required(true))
                .arg(Arg::with_name("file_path").required(true)),
        )
        .subcommand(
            SubCommand::with_name("pull")
                .about("Pull a .env or config file from the server")
                .arg(Arg::with_name("project_name").required(true))
                .arg(Arg::with_name("output_file").required(true))
                .arg(
                    Arg::with_name("force")
                        .long("force")
                        .short('f')
                        .help("Overwrite the output file if it already exists"),
                ),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Update an existing .env or config file on the server")
                .arg(Arg::with_name("project_name").required(true))
                .arg(Arg::with_name("file_path").required(true)),
        )
        .subcommand(
            SubCommand::with_name("delete")
                .about("Delete a .env or config file from the server")
                .arg(Arg::with_name("project_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("share")
                .about("For the ease of sharing, generate a link to the dotenvpull config file, which can be used to pull the project's .env file")
                .arg(Arg::with_name("project_id").required(true))
                .arg(
                    Arg::with_name("include-all-projects")
                        .long("include-all-projects")
                        .short('a')
                        .help("Use this flag if you wish to share all projects (files) inside of your config"),
                ),
        )
        .subcommand(
            SubCommand::with_name("getshared")
                .about("Pull a shared .env or config file from the server")
                .arg(Arg::with_name("share_code").required(true))
                .arg(Arg::with_name("project_id").required(true))
                .arg(Arg::with_name("api_url").required(true))
                .arg(Arg::with_name("encryption_key").required(true)),
        )
        .subcommand(SubCommand::with_name("list").about("List all projects in the local config"))
        .get_matches();

    let config = get_or_create_config()?;
    let api_url = config["api_url"]
        .as_str()
        .unwrap_or("http://localhost:8080")
        .to_string();

    match matches.subcommand() {
        Some(("push", sub_m)) => {
            let project_name = sub_m.value_of("project_name").unwrap();
            let file_path = sub_m.value_of("file_path").unwrap();
            push(&api_url, project_name, file_path).await?;
        }
        Some(("pull", sub_m)) => {
            let project_name = sub_m.value_of("project_name").unwrap();
            let output_file = sub_m.value_of("output_file").unwrap();
            let force = sub_m.is_present("force");
            pull(&api_url, project_name, output_file, force).await?;
        }
        Some(("update", sub_m)) => {
            let project_name = sub_m.value_of("project_name").unwrap();
            let file_path = sub_m.value_of("file_path").unwrap();
            update(&api_url, project_name, file_path).await?;
        }
        Some(("delete", sub_m)) => {
            let project_name = sub_m.value_of("project_name").unwrap();
            delete(&api_url, project_name).await?;
        }
        Some(("list", _)) => {
            list_projects(&config);
        }
        Some(("share", sub_m)) => {
            let project_id = sub_m.value_of("project_id").unwrap();
            let include_all_projects = sub_m.is_present("include-all-projects");

            share(&api_url, project_id, include_all_projects).await?;
        }
        Some(("getshared", sub_m)) => {
            let share_code = sub_m.value_of("share_code").unwrap();
            let project_id = sub_m.value_of("project_id").unwrap();
            let api_url = sub_m.value_of("api_url").unwrap();
            let encryption_key = sub_m.value_of("encryption_key").unwrap();
            getshared(api_url, share_code, project_id, encryption_key).await?;
        }
        _ => println!("Please use a valid subcommand. Use --help for more information."),
    }

    Ok(())
}

fn get_or_create_config() -> Result<Value, Box<dyn std::error::Error>> {
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

fn encrypt(data: &str, key: &[u8; 32]) -> String {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, data.as_bytes()).unwrap();
    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);
    general_purpose::STANDARD.encode(&result)
}

fn decrypt(encrypted_data: &str, key: &[u8; 32]) -> String {
    let encrypted_bytes = general_purpose::STANDARD.decode(encrypted_data).unwrap();
    let nonce = Nonce::from_slice(&encrypted_bytes[..12]);
    let ciphertext = &encrypted_bytes[12..];
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher.decrypt(nonce, ciphertext).unwrap();
    String::from_utf8(plaintext).unwrap()
}

async fn share(
    api_url: &str,
    project_id: &str,
    include_all_projects: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Project ID: {}", project_id);
    let config = get_or_create_config()?;

    let project_config = if include_all_projects {
        // Share all projects in the config
        config.clone()
    } else {
        // Check if the project exists in the config
        if !config.as_object().unwrap().contains_key(project_id) {
            println!("Error: Project '{}' not found in local config", project_id);
            return Ok(());
        }
        json!({ project_id: config[project_id].clone() })
    };

    // Generate share code and encryption key
    let share_code: [u8; 32] = rand::thread_rng().gen();
    let encryption_key: [u8; 32] = rand::thread_rng().gen();

    let share_code_str = general_purpose::STANDARD.encode(&share_code);

    let client = Client::new();
    // Send encrypted project config to the server
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

async fn getshared(
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

async fn push(
    api_url: &str,
    project_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let content = fs::read_to_string(file_path)?;

    let encryption_key: [u8; 32] = rand::thread_rng().gen();
    let encrypted_content = encrypt(&content, &encryption_key);

    let response = client
        .post(&format!("{}/store", api_url))
        .json(&json!({
            "project_id": project_name,
            "encrypted_content": encrypted_content
        }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("File pushed successfully");
        let json: Value = response.json().await?;
        if let Some(access_key) = json.get("access_key") {
            update_config(
                project_name,
                access_key.as_str().unwrap(),
                &general_purpose::STANDARD.encode(encryption_key),
            )?;
            println!("New access key and encryption key generated and stored");
        }
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}

async fn pull(
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
        .get(&format!("{}/retrieve", api_url))
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

async fn update(
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

async fn delete(api_url: &str, project_name: &str) -> Result<(), Box<dyn std::error::Error>> {
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

fn list_projects(config: &Value) {
    println!("Projects in local config:");
    for (project, _) in config.as_object().unwrap() {
        if project != "api_url" {
            println!("- {}", project);
        }
    }
}

fn update_config(
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
