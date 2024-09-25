mod api;
mod cli;
mod config;
mod crypto;
mod utils;

use crate::cli::run_cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_cli().await
}
