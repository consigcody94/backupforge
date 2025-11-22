.PHONY: help build test clean install run fmt clippy doc docker release

# Default target
help:
	@echo "BackupForge - Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build       - Build the project in debug mode"
	@echo "  release     - Build the project in release mode"
	@echo "  test        - Run all tests"
	@echo "  clean       - Clean build artifacts"
	@echo "  install     - Install backupforge binary"
	@echo "  run         - Run the CLI tool"
	@echo "  fmt         - Format code with rustfmt"
	@echo "  clippy      - Run clippy linter"
	@echo "  doc         - Generate documentation"
	@echo "  docker      - Build Docker image"
	@echo "  check       - Run fmt, clippy, and tests"

# Build in debug mode
build:
	cargo build

# Build in release mode
release:
	cargo build --release

# Run tests
test:
	cargo test --all --verbose

# Clean build artifacts
clean:
	cargo clean

# Install binary system-wide
install: release
	sudo cp target/release/backupforge /usr/local/bin/
	@echo "✓ Installed to /usr/local/bin/backupforge"

# Install binary to user directory
install-user: release
	mkdir -p $(HOME)/.local/bin
	cp target/release/backupforge $(HOME)/.local/bin/
	@echo "✓ Installed to $(HOME)/.local/bin/backupforge"
	@echo "  Make sure $(HOME)/.local/bin is in your PATH"

# Run the CLI
run:
	cargo run --bin backupforge -- $(ARGS)

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Run clippy
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

# Generate documentation
doc:
	cargo doc --no-deps --open

# Build Docker image
docker:
	docker build -t backupforge:latest .

# Docker compose up
docker-up:
	docker-compose up -d

# Docker compose down
docker-down:
	docker-compose down

# Run all checks
check: fmt-check clippy test
	@echo "✓ All checks passed"

# Run cargo audit
audit:
	cargo audit

# Update dependencies
update:
	cargo update

# Show dependency tree
tree:
	cargo tree

# Benchmark
bench:
	cargo bench

# Coverage (requires cargo-tarpaulin)
coverage:
	cargo tarpaulin --all-features --workspace --timeout 120 --out Html

# Watch for changes and rebuild
watch:
	cargo watch -x build

# Quick check (no tests)
quick-check: fmt-check clippy
	cargo check --all

# Setup development environment
setup:
	./scripts/setup.sh
