# DotEnvPull Backend  

This is a backend part, it's a simple Rust Actix Web server that exposes an /store /pull /update and /delete endpoints to interact with the encrypted content of the config.

As part of it - Mongo Database is used to store the encrypted content of the config.  

## How to run it
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Clone the repo
git clone https://github.com/denver-code/dotenv-pull.git  
cd dotenv-pull/backend
# Create a .env file with the following content:
# DATABASE_URL=mongodb://localhost:27017
# DATABASE_NAME=dotenv-pull
# SERVER_URL=127.0.0.1:8080
# Run the server
cargo run
# Or build and run
cargo build --release
# Find the executable in target/release
```