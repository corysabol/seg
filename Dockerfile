# Stage 1: Build the Rust binary
FROM rust:latest AS builder

WORKDIR /app
COPY . .

# Build the scanner binary in release mode
RUN cargo build --release

# Stage 2: Final image with nmap and binary
FROM debian:latest

# Install nmap
RUN apt-get update && apt-get install -y nmap && apt-get clean

# Copy the compiled Rust binary from the builder stage
COPY --from=builder /app/target/release/seg /usr/local/seg/bin/seg

# Run the scanner binary
ENTRYPOINT ["/usr/local/seg/bin/seg", "scan"]
