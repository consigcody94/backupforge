# Contributing to BackupForge

Thank you for your interest in contributing to BackupForge! We welcome contributions from everyone.

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates. When creating a bug report, include:

- **Clear title and description**
- **Steps to reproduce**
- **Expected behavior**
- **Actual behavior**
- **Environment details** (OS, Rust version, BackupForge version)
- **Relevant logs** (use `RUST_LOG=debug`)

### Suggesting Features

Feature requests are welcome! Please provide:

- **Clear use case**
- **Expected behavior**
- **Why this would be useful**
- **Possible implementation approach** (optional)

### Pull Requests

1. **Fork the repository**
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Make your changes**
4. **Add tests** for new functionality
5. **Ensure all tests pass** (`cargo test`)
6. **Format your code** (`cargo fmt`)
7. **Run clippy** (`cargo clippy -- -D warnings`)
8. **Commit your changes** (`git commit -m 'Add amazing feature'`)
9. **Push to your fork** (`git push origin feature/amazing-feature`)
10. **Open a Pull Request**

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- OpenSSL development libraries
- pkg-config

### Build

```bash
git clone https://github.com/consigcody94/backupforge.git
cd backupforge
cargo build
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for specific crate
cargo test -p backupforge-core
```

### Code Style

We use `rustfmt` and `clippy`:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings
```

### Documentation

```bash
# Build documentation
cargo doc --no-deps --open

# Check documentation
cargo doc --no-deps
```

## Project Structure

```
backupforge/
├── crates/
│   ├── common/      # Shared types and utilities
│   ├── core/        # Core backup engine
│   ├── storage/     # Storage backends
│   ├── agent/       # Backup agents
│   ├── server/      # API server
│   └── cli/         # CLI tool
├── docs/            # Documentation
├── scripts/         # Helper scripts
└── web-dashboard/   # Frontend (planned)
```

## Coding Guidelines

### General Principles

- **Keep it simple** - Prefer simple solutions over complex ones
- **Document public APIs** - Use rustdoc comments
- **Error handling** - Use `Result` types, provide context
- **Testing** - Write tests for new features
- **Performance** - Profile before optimizing

### Rust Conventions

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use meaningful variable names
- Prefer iterators over loops where appropriate
- Use `?` operator for error propagation
- Avoid `unwrap()` in production code

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add VM snapshot support
fix: resolve chunk deduplication race condition
docs: update installation guide
test: add integration tests for S3 backend
refactor: simplify encryption module
perf: optimize chunking performance
chore: update dependencies
```

### Documentation

- **Public functions**: Document with rustdoc (`///`)
- **Modules**: Add module-level documentation (`//!`)
- **Examples**: Include usage examples
- **Panics**: Document when functions can panic
- **Safety**: Document unsafe code

Example:

```rust
/// Chunks data using content-defined chunking (CDC).
///
/// This function splits the input data into variable-sized chunks
/// using a rolling hash algorithm for optimal deduplication.
///
/// # Arguments
///
/// * `data` - The data to chunk
///
/// # Returns
///
/// A vector of chunks with their hash identifiers.
///
/// # Examples
///
/// ```
/// use backupforge_core::Chunker;
///
/// let chunker = Chunker::new(ChunkingStrategy::default());
/// let chunks = chunker.chunk_data(b"Hello, World!").unwrap();
/// ```
pub fn chunk_data(&self, data: &[u8]) -> Result<Vec<Chunk>> {
    // ...
}
```

## Testing

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking() {
        let chunker = Chunker::new(ChunkingStrategy::default());
        let result = chunker.chunk_data(b"test");
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory.

### Test Coverage

We aim for >80% code coverage for critical paths.

## Performance

### Benchmarking

```bash
cargo bench
```

### Profiling

Use `cargo flamegraph` or `perf` for profiling.

## Documentation

### User Documentation

Located in `docs/`:
- Architecture diagrams
- User guides
- API documentation
- Troubleshooting guides

### Code Documentation

- Use rustdoc for all public APIs
- Include examples in documentation
- Keep documentation up-to-date

## Release Process

1. Update `CHANGELOG.md`
2. Bump version in `Cargo.toml`
3. Create git tag (`git tag -a v0.x.0 -m "Release v0.x.0"`)
4. Push tag (`git push origin v0.x.0`)
5. GitHub Actions will create release

## Questions?

- Open a [Discussion](https://github.com/consigcody94/backupforge/discussions)
- Ask in [Issues](https://github.com/consigcody94/backupforge/issues)

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

## Recognition

Contributors will be recognized in:
- `README.md` contributors section
- Release notes
- Project documentation

Thank you for contributing to BackupForge! 🚀
