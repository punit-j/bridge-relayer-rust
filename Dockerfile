FROM rust:latest
WORKDIR /app
COPY . .
RUN cargo build
CMD cargo run -- --config /path/to/config.json
