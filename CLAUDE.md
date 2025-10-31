# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`audit-box` is a command-line TUI tool for managing overlay filesystems. It displays diffs and new files present in an overlay filesystem, allowing users to selectively write changes back to the main filesystem.

## Technology Stack

- **Language**: Rust
- **Interface**: Terminal User Interface (TUI)

## Common Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run

# Run with arguments
cargo run -- [args]

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Run clippy with all features
cargo clippy --all-features
```

## Architecture

### Core Components

- **Filesystem Layer**: Interacts with overlay filesystem, reads differences between layers
- **Diff Engine**: Compares files between overlay and base filesystem, generates diffs
- **TUI Layer**: Renders interactive interface for viewing diffs and selecting files
- **Write-back Logic**: Handles selective copying/merging of files from overlay to base filesystem

### Key Functionality

- Scan overlay filesystem to identify new and modified files
- Generate and display file diffs
- Provide interactive selection interface
- Safely write selected changes back to base filesystem
