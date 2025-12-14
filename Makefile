# Define the default target
.PHONY: all
all: build

# Build the Rust project
.PHONY: build
build:
	@echo "Building the project"
	cargo build --release --target x86_64-unknown-linux-musl

# Run the Rust project
.PHONY: run
run:
	@echo "Running the project with RUST_LOG='$(RUST_LOG)'"
	RUST_LOG=$(RUST_LOG) cargo run

# Clean the project
.PHONY: clean
clean:
	cargo clean

# Test the project
.PHONY: test
test:
	@echo "Running tests..."
	cargo test

# Format the code
.PHONY: format
format:
	cargo fmt

# Check the code
.PHONY: check
check:
	cargo clippy
	cargo check

# Prepare the sqlx
.PHONY: prepare
prepare:
	cargo sqlx prepare

# Run migrations
.PHONY: migrate
migrate:
	cargo sqlx migrate run

# Generate config.toml
.PHONY: bucket
bucket:
	@echo "Configuring minio client..."
	mc alias set --quiet mc "http://127.0.0.1:9000" "minioadmin" "minioadmin"
	@echo "Creating bucket..."
	mc mb --region global --ignore-existing --quiet mc/pastebin
	@echo "Setting up access key..."
	mc admin accesskey create \
		--access-key '<s3 access key!>' \
		--secret-key '<s3 secret key!>' \
		--policy confs/s3/key-policy.json mc/
	@echo "Configuring anonymous access..."
	mc anonymous set download mc/pastebin
	@echo "Configuration complete"
