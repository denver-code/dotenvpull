use crate::api::{delete, getshared, pull, share, update};
use crate::config::get_or_create_config;
use crate::utils::list_projects;
use clap::{App, Arg, SubCommand};

pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
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
            crate::api::push(&api_url, project_name, file_path).await?;
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
