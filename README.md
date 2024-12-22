# ada-pastebin
Hi! This is a little pastebin application with colourful HTML support!

## Prerequisites

Building and running the thing has a few requirements:
 * A working [Rust](https://rustup.rs/) environment
 * An account with a PostgreSQL server with credentials in.. an environment variable called DATABASE_URL
 * An account with [Amazon S3](https://aws.amazon.com/s3/) or a [compatible service](https://www.s3compare.io/), credentials in config.toml
 * An account with [Cloudflare Turnstile](https://www.cloudflare.com/application-services/products/turnstile/) with site key and secret key noted in config.toml

 Optional, nice to have things but not strictly required:
 <!-- * An account with [Google Cloud Platform](https://cloud.google.com/) with [Google Drive API (v3)](https://developers.google.com/drive/) enabled, credentials in config.toml -->
 * An account with [Cloudflare](https://www.cloudflare.com/) with an API Token scoped for `Zone.Cache Purge`, credentials in config.toml
 * An application registered with [Discord](https://discord.dev/), credentials in config.toml

## How to use

 * Clone the repository with `git clone https://github.com/adayoung/ada-pastebin`
 * Copy config.toml.sample to a file called config.toml and edit for correct values
 * Export the DATABASE_URL in the form `DATABASE_URL=postgres://<username>:<password>@<host>/<db>`
 * Setup database with `cargo install sqlx-cli` and then `sqlx db create` followed with `sqlx migration run`
 * Run it with `cargo run` (or build it with `cargo build --release`)
 * Point your browser to http://localhost:2024/
