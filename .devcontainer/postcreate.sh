#!/usr/bin/env bash

rustup target add x86_64-unknown-linux-musl
cargo install --no-default-features -F postgres,rustls sqlx-cli
make migrate
make bucket
