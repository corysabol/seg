FROM rust:latest AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

# Stage 2: Final image with nmap and binary
FROM debian:latest

RUN apt-get update && apt-get install -y nftables nmap && apt-get clean

COPY --from=builder /app/target/release/seg /usr/local/seg/bin/seg

ENTRYPOINT ["/usr/local/seg/bin/seg"]
