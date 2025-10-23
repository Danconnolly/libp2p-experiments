# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **libp2p experiments project** - a Rust-based exploration of the libp2p peer-to-peer networking library. The project demonstrates basic libp2p functionality by creating a simple P2P application that can participate in distributed networks using publish/subscribe messaging patterns (Floodsub).

## Architecture

### High-Level Structure

**Single Binary Workspace:**
- `bin/p2p/` - Main P2P application entry point
  - Uses Cargo workspace resolver v2
  - Single executable: `p2p`

### Core Architecture Pattern

The application follows an **event-driven async architecture** using Tokio:

1. **CLI Interface** (Clap) - Accepts `--verbose` flag for debug logging
2. **Logging System** (Tracing) - Configurable structured logging
3. **P2P Network Stack** (libp2p)
   - Transport: TCP with TLS + Noise encryption protocols
   - Multiplexing: Yamux for stream multiplexing
   - Messaging: Floodsub publish/subscribe
   - Identity: Ed25519 keypair for peer identity
4. **Event Loop** (Tokio async runtime) - Continuous polling of swarm events

### Execution Flow

```
CLI Parsing → Logging Init → Identity Generation →
Floodsub Config → Swarm Setup → Event Loop (infinite)
```

The event loop continuously polls the libp2p swarm for network events and prints them to stdout. This is an experimental/learning implementation.

## Development Commands

### Building
```bash
cargo build                    # Debug build
cargo build --release         # Release build with optimizations
```

### Running
```bash
cargo run -- --verbose        # Run with debug logging
cargo run                     # Run with info-level logging
```

### Testing & Quality
```bash
cargo test                    # Run tests (if any)
cargo fmt                     # Format code (always run before commits)
cargo clippy                  # Check for common mistakes and suggest improvements
```

**Important:** Before pushing commits, run both `cargo fmt` and `cargo clippy`. See "Pre-commit Workflow" below.

### Dependency Management
```bash
cargo update                  # Update dependencies within Cargo.lock constraints
cargo tree                    # View dependency tree
cargo outdated                # Check for newer versions (requires cargo-outdated)
```

## Key Dependencies

| Dependency | Version | Purpose |
|---|---|---|
| libp2p | 0.56 | Core P2P networking framework |
| tokio | 1.48 | Async runtime |
| tracing | 0.1 | Structured logging |
| clap | 4.5 | CLI argument parsing |
| anyhow | 1.0 | Error handling |
| libp2p-tls | 0.6.2 | TLS transport security |
| libp2p-noise | 0.46.1 | Noise protocol security |
| libp2p-yamux | 0.47.0 | Stream multiplexing |

## Important Patterns

### Async/Await
The entire codebase uses Tokio's async/await. All network operations are non-blocking.

### Error Handling
Uses `anyhow::Result<T>` for ergonomic error handling throughout.

### Logging
- Use `tracing::debug!`, `tracing::info!`, etc. for logging
- Verbose flag (--verbose) sets log level to DEBUG
- Structured logging with environment filters via `tracing-subscriber`

## Pre-commit Workflow

Before committing Rust code:

1. **Format code:** `cargo fmt`
2. **Run linter:** `cargo clippy`
3. **Address any warnings** from clippy
4. **Create commit with message ending:** Include the Claude Code footer

**Git Safety:** If you see `Error: git@github.com: Permission denied (publickey)`, the push will not be possible. Do not proceed.

## Code Organization Notes

- **Currently monolithic:** All logic is in `bin/p2p/src/main.rs` (~64 lines)
- **No separate library:** The workspace contains only the binary crate
- **Future modularity:** As the project grows, consider extracting networking logic into separate modules

## Running Individual Features

### Enable Debug Logging
```bash
cargo run -- --verbose
```

### Check Specific Clippy Rules
```bash
cargo clippy -- -W clippy::all
```

### Generate Documentation
```bash
cargo doc --open
```

## Workspace Configuration

- **Edition:** Rust 2024
- **Resolver:** Version 2 (better dependency handling)
- **Lock File:** `Cargo.lock` (committed for reproducibility)

## Useful Resources for Development

- libp2p documentation: https://docs.rs/libp2p/
- Tokio async runtime: https://tokio.rs/
- Rust tracing guide: https://docs.rs/tracing/
