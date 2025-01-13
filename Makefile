# Define the default target
.PHONY: all
all: build

# Build the Rust project
.PHONY: build
build:
	@echo "Building the project"
	cargo build --release

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
