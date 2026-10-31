# --- Build ---
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Cachea las dependencias: compila un main vacío primero
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

# --- Runtime ---
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/file_uploader /usr/local/bin/file_uploader
RUN mkdir -p /app/files
EXPOSE 3050
CMD ["file_uploader"]
