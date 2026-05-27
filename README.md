# MentorFish — GM Training System

> Personal chess improvement platform with AI-powered game analysis, opening management, and an adaptive training engine.

## Prerequisites

| Tool         | Version    | Notes                              |
| ------------ | ---------- | ---------------------------------- |
| Rust         | 1.77+      | [rustup](https://rustup.rs)        |
| Node.js      | 22+        | [nodejs.org](https://nodejs.org)   |
| Stockfish    | 16+        | Chess engine for position analysis |
| Ollama       | Latest     | Local LLM serving (embeddings)     |
| PostgreSQL   | 16+        | Relational database                |

### Windows-specific

The [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) or Visual Studio with the "Desktop development with C++" workload is required. The Windows CI runner already includes the MSVC toolchain.

## Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/<your-org>/mentorfish.git
cd mentorfish

# 2. Install frontend dependencies
npm install

# 3. Start the development server (frontend + Tauri)
just dev
```

> **Note:** Tauri will compile the Rust backend on first launch. Subsequent runs use incremental compilation.

## Available Tasks

Run `just --list` to see all available tasks:

```
just dev              # Start dev server (Tauri + frontend)
just build            # Build release binary
just test             # Run all Rust unit tests
just test-verbose     # Run Rust tests with full output
just test-filter NAME # Run tests matching NAME
just lint             # Lint frontend (ESLint)
just fmt              # Format Rust code
just fmt-check        # Check Rust formatting (CI)
just check            # Fast Rust compilation check (no binary)
just clippy           # Run Clippy lints (deny warnings)
just watch            # Auto check + test on file changes
just clean            # Remove all build artifacts
just update           # Update Rust + Node dependencies
just ingest           # Process PDFs into vector DB
just openings DIR     # Build opening tree from PGN files
just merge-trees      # Merge multiple opening trees
```

## Project Structure

```
mentorfish/
├── src/                   # React + TypeScript frontend
│   ├── components/        # Reusable UI components
│   ├── stores/            # Zustand state stores
│   └── ...
├── src-tauri/             # Rust backend (Tauri)
│   ├── src/               # Rust source code
│   ├── migrations/        # SQLx database migrations
│   ├── binaries/          # Bundled external binaries
│   └── Cargo.toml         # Rust dependencies
├── scripts/               # Python utility scripts
│   ├── ingest.py          # PDF knowledge ingestion
│   ├── build_openings.py  # PGN → opening tree builder
│   ├── merge_trees.py     # Opening repertoire merger
│   └── parse_abk.py       # Arena Book parser
├── knowledge/             # Training data (PGN, PDFs)
├── docs/                  # Documentation
├── justfile               # Dev task runner
└── .github/workflows/     # CI/CD pipelines
    ├── ci.yml             # Lint, test, build on PR
    └── release.yml        # Windows release on version tag
```

## CI/CD

- **CI** — Runs on every push/PR to `main`: ESLint, `cargo test`, `cargo check`, and Vite build.
- **Release** — Triggered by `v*` tags. Builds the Windows x64 installer via `tauri-action` and uploads it as a GitHub Release asset.
- **Dependabot** — Weekly updates for Cargo, npm, and GitHub Actions.

## License

Proprietary — all rights reserved.
