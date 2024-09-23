# Use the official Rust image as a parent image
FROM rust:1.70 as builder

# Set the working directory in the container
WORKDIR /usr/src/app

# Copy the current directory contents into the container
COPY . .

# Build the application
RUN cargo build --release

# Start a new stage for a smaller final image
FROM ubuntu:latest

# Install OpenSSL - often needed for Rust applications
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/dotenvpull-backend /usr/local/bin/dotenvpull-backend

# Copy the .env file
COPY .env /usr/local/bin/.env

# Set the working directory
WORKDIR /usr/local/bin

# Set the startup command
CMD ["./backend"]
