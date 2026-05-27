# MentorFish Development Tasks
# Install just: cargo install just

default:
    @just --list

# ─── Development ─────────────────────────────────────────────────

# Start development server (frontend + Tauri hot-reload)
dev:
    npm run tauri:dev

# Watch mode — auto-check + test on file changes
watch:
    cargo watch -C src-tauri -x check -x test

# ─── Build ──────────────────────────────────────────────────────

# Build release binary (Tauri + frontend)
build:
    npm run tauri:build

# Build frontend only (TypeScript + Vite)
build-frontend:
    npm run build

# ─── Test ───────────────────────────────────────────────────────

# Run all Rust unit tests
test:
    cd src-tauri && cargo test

# Run all tests with full output
test-verbose:
    cd src-tauri && cargo test -- --nocapture

# Run a specific test by name filter
test-filter filter:
    cd src-tauri && cargo test {{filter}}

# ─── Static Analysis ────────────────────────────────────────────

# Check Rust compilation (fast, no binary output)
check:
    cd src-tauri && cargo check

# Lint frontend (ESLint)
lint:
    npm run lint

# Format all Rust code
fmt:
    cd src-tauri && cargo fmt

# Format check (CI mode — exit 1 if not formatted)
fmt-check:
    cd src-tauri && cargo fmt --check

# Run Clippy lints
clippy:
    cd src-tauri && cargo clippy -- -D warnings

# ─── Knowledge Processing ───────────────────────────────────────

# Ingest all PDFs from knowledge/books/ into the vector database
ingest:
    python scripts/ingest.py --all

# Build opening tree from PGN files (default: knowledge/pgn/)
openings pgn_dir="knowledge/pgn":
    python scripts/build_openings.py {{pgn_dir}}

# Build opening tree from ABK files (full pipeline: parse + merge)
build-openings abk_dir="knowledge/openings":
    @echo "=== Building Opening Tree from ABK files ==="
    @for abk in {{abk_dir}}/*.abk; do
        @if [ -f "$$abk" ]; then
            echo "Parsing $$(basename "$$abk" .abk)..."
            python scripts/parse_abk.py "$$abk" --output "knowledge/$$(basename "$$abk" .abk)_tree.json"
        fi
    done
    @echo ""
    @echo "=== Merging trees ==="
    python scripts/merge_trees.py knowledge/*_tree.json --output knowledge/openings_tree_merged.json

# Merge multiple opening trees into a combined repertoire
merge-trees:
    python scripts/merge_trees.py

# Parse ABK (Arena Book) files into opening trees
parse-abk file:
    python scripts/parse_abk.py {{file}}

# ─── Maintenance ────────────────────────────────────────────────

# Clean all build artifacts (Rust + Vite cache)
clean:
    cd src-tauri && cargo clean
    rm -rf node_modules/.vite

# Update all dependencies
update:
    cargo update --manifest-path src-tauri/Cargo.toml
    npm update
