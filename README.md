[![Rust](https://github.com/adayoung/ada-pastebin/actions/workflows/rust.yml/badge.svg)](https://github.com/adayoung/ada-pastebin/actions/workflows/rust.yml)

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

## How to use (with Codespaces)
* Be signed into GitHub and click on the green `Code` button in the top right corner of the repository
* Select Codespaces tab and click on the `Create codespace`
* Let the thing wriggle and build for a bit! It takes a while for the first time
* Click on forwarded ports and open the address for port 9001 in your browser
* Input `minioadmin` for username and password in it!
* Make a new bucket called `pastebin` in there and copy `confs/s3/bucket-policy.json` into its custom access policy
* Make a new key in there and copy `confs/s3/key-policy.json` into its user policy, make sure to note down the access key and secret key!
* Copy config.toml.sample to a file called config.toml and edit for correct values
* The codespace domain for port 2024 goes in `static_domain` in config.toml
* The codespace address+`pastebin/` for port 9000 goes in `s3_bucket_url` in config.toml
* Open the terminal and type `make migrate`, `make check`, and then `make run`
* Open the address for the forwarded port 2024 in your browser!
* Tada!

## How to use (Old school way)

 * Clone the repository with `git clone https://github.com/adayoung/ada-pastebin`
 * Copy config.toml.sample to a file called config.toml and edit for correct values
 * Export the DATABASE_URL in the form `DATABASE_URL=postgres://<username>:<password>@<host>/<db>`
 * Setup database with `cargo install sqlx-cli` and then `sqlx db create` followed with `sqlx migrate run`
 * Run it with `cargo run` (or build it with `cargo build --release`)
 * Point your browser to http://localhost:2024/

## Icons

We have pretty icons from [Feather!](https://feathericons.com/)
