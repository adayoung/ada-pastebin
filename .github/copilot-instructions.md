# Guidance for AI coding agents working on ada-pastebin

Short, actionable notes to help an AI contributor be productive quickly.

- **Big picture:** This repository is a Rust web service (axum) that serves a pastebin-style app. Key responsibilities are: HTTP routes and handlers (`src/main.rs`, `src/api.rs`, `src/paste.rs`), HTML rendering via Askama templates (`src/templates.rs`, `templates/*.j2`), persistent data via Postgres (`migrations/` + `sqlx`), and content storage in S3 (`src/s3.rs`, `confs/s3/`). Background tasks (view updates, Cloudflare cache cleanup) run via tokio tasks in `src/main.rs` and `src/paste.rs`.

- **Where to read first:** `src/main.rs` (routing + startup), `src/config.rs` (config loading; supports `config.toml` and `APP_` env vars), `src/runtime.rs` (shared AppState), and `src/templates.rs` (Askama structs that map to `templates/*.j2`).

- **Build & run (local):**
  - Build: `cargo build --release`
  - Run: set at least `DATABASE_URL` and optionally `RUST_LOG`; then `cargo run --release` or run the built binary.
  - The project provides Makefile shortcuts: `make build`, `make run`, `make migrate`.

- **Database & migrations:**
  - Migrations live in `migrations/` and are executed with `sqlx`.
  - Use `cargo sqlx prepare` and `cargo sqlx migrate run` or `make prepare` / `make migrate` with `DATABASE_URL` set.

- **Configuration conventions:**
  - `src/config.rs` reads `config.toml` if present (see `config.toml.sample`) and then overlays environment variables with prefix `APP_` (e.g., `APP_STATIC_DOMAIN`).
  - Secrets like cookie keys, AWS creds, and recaptcha are configured via `config.toml` or `APP_` env vars.

- **Static assets & templates:**
  - Templates are Askama (`templates/*.j2`) and mapped by `src/templates.rs`. Template structs determine the fields available in templates (e.g., `PasteTemplate`).
  - Static files are embedded via `rust-embed` and served by `src/static_files.rs`. Files under `static/vendor/` are pre-compressed with Brotli (`*.br`) and `static_files.rs` trims `.br` and sets `Content-Encoding: br`.

- **Security & sessions:**
  - CSRF is enforced with `axum_csrf` (configured in `src/main.rs`) and expects tokens in forms.
  - Cookies use `tower-cookies` and private cookies are signed with `cookie_key` from config.
  - Sessions/paste ownership rely on a combination of DB-backed user IDs and session cookies (`src/session.rs`). Check migrations for relevant tables (`migrations/*sessions*.sql`).

- **External integrations to be careful with:**
  - AWS S3 via `aws-sdk-s3` (`src/s3.rs`) — bucket and prefix are in config.
  - Cloudflare cache purge (`src/cloudflare.rs`) — configured by `cloudflare_enabled` and `cloudflare_purge_url`.
  - OAuth for Discord and Google Drive (`src/discord.rs`, `src/gdrive.rs`, `src/oauth.rs`) — follow `src/config.rs` `OauthConfig` shapes.

- **Code patterns & idioms:**
  - Web handlers use `axum` extractors and return Askama templates or JSON responses. Look at `newpaste`, `getpaste`, `editpaste` in `src/main.rs` for examples.
  - Database access is via `sqlx` with `PgPool` stored in shared `AppState` (`src/runtime.rs`). Use `&state.db` for queries.
  - Background tasks are spawned with `tokio::spawn` at startup (see `main.rs`), so changes that affect startup should consider graceful shutdown handling.

- **Tests & linting:**
  - The repository includes no heavy test harness — use `cargo test` and `cargo clippy` / `cargo fmt` (Makefile targets `test`, `check`, `format`).

- **Editing guidance (practical examples):**
  - To add a new route that renders a template: add handler in `src/main.rs`, create a template struct in `src/templates.rs` referencing the `.j2` file, and add the `.j2` under `templates/`.
  - To serve a new static vendor file: add the file to `static/` (optionally precompress to `.br`), then reference it under `/static/*path` (served by `src/static_files.rs`).
  - To modify DB schema: add a new migration under `migrations/` and run `make migrate` with `DATABASE_URL` set.

- **Common pitfalls to avoid:**
  - Don’t forget CSRF tokens for form handlers — handlers validate tokens via `axum_csrf` (see `newpaste`).
  - Static `.br` files are embedded; editing `static/` requires re-building to update the binary.
  - Configuration defaults in `src/config.rs` may hide missing config values — prefer creating a `config.toml` during local dev.

