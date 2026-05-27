# MentorFish — GM Training System

A Tauri 2 desktop chess improvement platform with an AI-powered training engine. React 19 + TypeScript 6 frontend, Rust 2021 backend, Stockfish for analysis, Ollama for local LLM inference, and PostgreSQL for persistence.

## Commands

Use `just` as the task runner. All commands are copy-pasteable.

```bash
just dev              # Start Tauri dev server (frontend + Rust backend)
just build            # Release build (TypeScript + Vite + Tauri bundle)
just build-frontend   # Frontend-only build (tsc -b && vite build)
just lint             # ESLint on frontend
just fmt              # Format Rust code (cargo fmt)
just fmt-check        # Check Rust formatting (CI)
just check            # Fast Rust compilation check (cargo check, no binary)
just clippy           # Clippy lints (deny warnings)
just test             # Run all Rust unit tests
just test-filter NAME # Run Rust tests matching NAME
just clean            # Remove all build artifacts
```

## Project Structure

```
src/                  # React frontend
  components/         # Feature-scoped UI: Analysis, Board, Chat, Dashboard, Explorer, Library, Play, Settings, Layout
  stores/             # Zustand state stores (index.ts barrel)
  hooks/              # Custom React hooks
  lib/                # Tauri IPC bridge (tauriBridge.ts), shared TypeScript types (types.ts)
src-tauri/            # Rust backend (Tauri)
  src/
    agents/           # AI agent implementations
    database/         # SQLx queries and DB access layer
    engine/           # Stockfish chess engine integration
    features/         # Feature-specific backend modules
    inference/        # Ollama LLM client (ollama.rs)
    ipc/              # Tauri command handlers
    knowledge/        # Vector DB / embedding knowledge retrieval
    memory/           # Conversation memory / context
    orchestrator/     # Training orchestration logic
  migrations/         # SQLx database migrations
  binaries/           # Bundled external binaries (Stockfish)
scripts/              # Python utilities (ingest, opening trees, ABK parser)
knowledge/            # Training data: PGN files, PDF books
docs/                 # Documentation (PRD, architecture)
```

## Tech Stack

| Layer | Technology | Version | Notes |
|-------|-----------|---------|-------|
| Desktop shell | Tauri | 2 | Rust-based, bundles Stockfish |
| Frontend framework | React | 19.2 | Vite 8 dev server (:5173) |
| Language | TypeScript | 6.0 | Strict mode, verbatimModuleSyntax |
| Styling | Tailwind CSS | 4.3 | Vite plugin, no PostCSS config |
| State management | Zustand | 5.0 | Lightweight, stores in `src/stores/` |
| Chess UI | Chessground | 10.1 | @lichess-org/chessground |
| Chess logic (TS) | chess.js | 1.0-beta.8 | Move validation, PGN parsing |
| Chess logic (Rust) | shakmaty | 0.28 | Server-side move generation |
| Chess engine | Stockfish | 16+ | Bundled binary via `binaries/` |
| Database | PostgreSQL | 16+ | SQLx with Rustls, UUID, Chrono |
| Cache | Redis | 0.27 | tokio-comp feature |
| LLM | Ollama | latest | nomic-embed-text-v1.5 for embeddings |
| Task runner | just | — | `just --list` for available tasks |
| Lint (TS) | ESLint | 10.3 | typescript-eslint recommended |
| Lint (Rust) | Clippy | — | `-D warnings` (treat warnings as errors) |
| Format (Rust) | rustfmt | — | Standard edition 2021 style |

## Code Conventions

### TypeScript / React

- **Strict TypeScript**: `noUnusedLocals`, `noUnusedParameters`, `verbatimModuleSyntax` enabled. Never leave unused imports.
- **Components**: One component per file, named export preferred. Feature folders under `src/components/<Feature>/`.
- **Imports**: Use relative paths within `src/`. No path aliases configured.
- **State**: Use Zustand stores in `src/stores/`. Barrel export from `index.ts`.
- **Tauri bridge**: All IPC calls go through `src/lib/tauriBridge.ts`. Do not call `@tauri-apps/api` directly from components.
- **Types**: Shared frontend types in `src/lib/types.ts`. Do not redefine types from the Rust backend — keep them in sync manually.
- **Styling**: Tailwind utility classes. No CSS modules. Global styles in `src/index.css`. Chessground themes in `chessground.*.css`.

### Rust

- **Edition 2021**, minimum Rust version 1.77.2.
- **Error handling**: Use `anyhow` for application errors, `thiserror` for library error types. Prefer `?` over unwrap.
- **Async runtime**: Tokio (full features). All I/O should be async.
- **Database**: All queries through SQLx in `src-tauri/src/database/`. Migrations in `src-tauri/migrations/`. Never write raw SQL outside the database module.
- **Tauri commands**: Defined in `src-tauri/src/ipc/`. Register in `lib.rs`.
- **Logging**: Use the `log` crate + `tauri-plugin-log`. No `println!` in production code.
- **Formatting**: `cargo fmt` (standard rustfmt). Enforced in CI with `--check`.

## Testing

```bash
just test             # Run all Rust unit tests
just test-filter NAME # Run tests matching NAME
just test-verbose     # Run with --nocapture for full output
```

- Rust tests use `#[cfg(test)]` modules within source files. No separate test directory.
- Frontend has no test framework configured yet. When adding one, prefer Vitest.
- CI runs `cargo test` on every push/PR.

## Boundaries

### 🚫 Never modify
- `src-tauri/target/` — build artifacts, gitignored
- `node_modules/` — dependency cache, gitignored
- `src-tauri/gen/` — Tauri-generated code (schemas, bindings)
- `src-tauri/binaries/stockfish*` — bundled engine binaries
- `knowledge/` — training data, treat as read-only unless explicitly asked
- `.github/workflows/` — CI config, ask before changing pipeline behavior

### ⚠️ Ask first
- `src-tauri/migrations/` — existing migrations. New migrations are fine; editing existing ones can break databases.
- `src-tauri/Cargo.toml` — adding or upgrading Rust dependencies
- `tauri.conf.json` — changing window config, bundle settings, or security policy
- `eslint.config.js` — changing lint rules

### ✅ Always do
- Add tests for new Rust functionality
- Run `just fmt` and `just clippy` before committing Rust changes
- Run `just lint` before committing frontend changes
- Write clear commit messages describing what and why
- Use subagents, and use mcps for gathering information from external services especially Context7 and Exa MCP tools

## Context & Architecture

- **What it does**: MentorFish is a personal chess training platform. It analyzes your games with Stockfish, manages opening repertoires, provides AI-powered explanations via Ollama, and generates adaptive training exercises.
- **Notable**: This is a desktop app (Tauri), not a web app. It bundles Stockfish as an external binary. The `orchestrator/` module is the brain — it coordinates agents, engine analysis, and knowledge retrieval into training plans.
- **External services**: Ollama must be running locally for LLM features. PostgreSQL and Redis are expected to be available. See `docs/PRD.md` for product requirements.
- **CI/CD**: GitHub Actions. `ci.yml` runs on push/PR to `main`. `release.yml` builds Windows installer on version tags (`v*`).

## Git Workflow

- Branch from `main`, PR back to `main`.
- Run `just fmt-check` and `just clippy` before pushing Rust changes.
- Run `just lint` before pushing frontend changes.
- CI must pass before merging.
- Release tags follow `v*` pattern (e.g., `v0.2.0`).
