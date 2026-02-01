FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN apt-get update && apt-get install -y musl-tools \
    && rustup target add x86_64-unknown-linux-musl \
    && cargo build --release --target=x86_64-unknown-linux-musl --bin ada-pastebin

# We do not need the Rust toolchain to run the binary!
FROM gcr.io/distroless/static-debian13 AS runtime

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/ada-pastebin ./
COPY config.toml ./

EXPOSE 2024
ENV APP_BIND_ADDR="0.0.0.0"
ENTRYPOINT ["/app/ada-pastebin"]
