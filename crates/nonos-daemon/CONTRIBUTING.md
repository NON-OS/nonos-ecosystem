# Contributing to NONOS Daemon

Thank you for your interest in contributing to NONOS Daemon!

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/NON-OS/nonos.git
cd nonos

# Build the daemon
cargo build -p nonos-daemon

# Run tests
cargo test -p nonos-daemon

# Run with debug logging
RUST_LOG=debug cargo run -p nonos-daemon -- run
```

## Code Style

### File Organization

- Maximum **400 lines** per file
- Minimal `mod.rs` files (~20-30 lines for exports only)
- Split large modules into focused submodules

### Directory Structure

```
src/
├── main.rs           # Entry point only (~100 lines)
├── lib.rs            # Library exports
├── cli/              # CLI handlers (one file per command group)
│   ├── mod.rs        # Exports only
│   ├── commands.rs   # CLI struct definitions
│   ├── run.rs        # Run command handler
│   ├── identity.rs   # Identity commands
│   └── ...
├── api/              # HTTP API
│   ├── mod.rs        # Exports only
│   ├── server.rs     # Server setup
│   ├── handlers.rs   # Request handlers
│   └── ...
└── ...
```

### Rust Guidelines

- Use `rustfmt` for formatting
- Run `cargo clippy` and fix all warnings
- Write doc comments for public APIs
- Prefer explicit error handling over `.unwrap()`
- Use meaningful variable names

### License Headers

All source files must include the AGPL-3.0 license header:

```rust
// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>
```

## Making Changes

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Make Changes

- Write clean, documented code
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test -p nonos-daemon

# Run specific test
cargo test -p nonos-daemon test_name

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -p nonos-daemon
```

### 4. Commit

Write clear, descriptive commit messages:

```
feat(cli): add identity export command

- Add export subcommand to identity CLI
- Support base64-encoded backup files
- Add --output flag for custom paths
```

Commit message prefixes:
- `feat:` New feature
- `fix:` Bug fix
- `refactor:` Code refactoring
- `docs:` Documentation only
- `test:` Adding tests
- `chore:` Build/tooling changes

### 5. Submit Pull Request

- Fill out the PR template
- Link related issues
- Ensure CI passes

## Architecture Overview

### Core Components

| Module | Purpose |
|--------|---------|
| `cli/` | Command-line interface handlers |
| `api/` | HTTP REST API server |
| `p2p/` | libp2p networking layer |
| `contracts/` | Ethereum contract interactions |
| `services/` | Background node services |
| `privacy/` | ZK identity and cache mixing |

### Key Dependencies

- **tokio** - Async runtime
- **clap** - CLI argument parsing
- **libp2p** - P2P networking
- **ethers** - Ethereum interactions
- **axum** - HTTP server
- **sled** - Embedded database

## Testing

### Unit Tests

Place tests in a `tests.rs` file within each module:

```rust
// src/api/tests.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler() {
        // ...
    }
}
```

### Integration Tests

Place in `tests/` directory at crate root:

```
nonos-daemon/
├── src/
└── tests/
    └── integration_test.rs
```

## Documentation

- Update README.md for user-facing changes
- Add doc comments to public APIs
- Include examples in documentation

## Questions?

- Open a [GitHub Discussion]
- Join our community chat

## License

By contributing, you agree that your contributions will be licensed under the AGPL-3.0 license.
