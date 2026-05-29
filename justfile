# MentorFish — Task Runner
# Run `just --list` for available commands

# Default: list available commands
default:
    @just --list

# Start Tauri dev server (frontend + Rust backend)
dev:
    cargo tauri dev

# Release build (TypeScript + Vite + Tauri bundle)
build:
    cargo tauri build

# Frontend-only build (tsc -b && vite build)
build-frontend:
    npx tsc -b && npx vite build

# ESLint on frontend
lint:
    npx eslint src/

# Format Rust code
fmt:
    cargo fmt

# Check Rust formatting (CI)
fmt-check:
    cargo fmt --check

# Fast Rust compilation check (no binary)
check:
    cargo check

# Clippy lints (deny warnings)
clippy:
    cargo clippy -- -D warnings

# Run all Rust unit tests
test:
    cargo test

# Run Rust tests matching NAME
test-filter NAME:
    cargo test {{NAME}}

# Run tests with full output
test-verbose:
    cargo test -- --nocapture

# Remove all build artifacts
clean:
    cargo clean
    rm -rf dist/
    rm -rf node_modules/.vite
