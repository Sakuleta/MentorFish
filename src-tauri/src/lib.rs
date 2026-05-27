pub mod agents;
pub mod config;
pub mod database;
pub mod engine;
pub mod features;
pub mod inference;
pub mod ipc;
pub mod knowledge;
pub mod memory;
pub mod orchestrator;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

// ─── Core Domain Types ───

pub type FEN = String;
pub type UCIMove = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Move {
    pub uci: UCIMove,
    pub san: Option<String>,
    pub move_number: u32,
    pub color: Color,
    pub fen_before: FEN,
    pub fen_after: FEN,
    pub eval_cp_before: Option<i32>,
    pub eval_cp_after: Option<i32>,
    pub eval_swing: Option<i32>,
    pub move_time_ms: Option<u32>,
    pub classification: Option<MoveClassification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MoveClassification {
    Best,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

pub type ConfidenceScore = f64;

// ─── Knowledge Directory Resolution ───

/// Resolve the knowledge directory path.
///
/// Tries several strategies in order:
/// 1. Relative to the current working directory (development from project root).
/// 2. Parent of CWD (when CWD is `src-tauri/`).
/// 3. Relative to the Tauri resource directory (production bundles).
///
/// Falls back to a relative `"knowledge"` path if nothing else works.
fn resolve_knowledge_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    // Sentinel file that should exist in the knowledge directory
    let sentinel = "openings_tree_merged.json";

    if let Ok(cwd) = std::env::current_dir() {
        // Try CWD/knowledge (project root in dev mode)
        let candidate = cwd.join("knowledge");
        if candidate.join(sentinel).exists() {
            log::info!("Knowledge dir found at CWD/knowledge: {:?}", candidate);
            return candidate.canonicalize().unwrap_or(candidate);
        }
        // Try CWD/../knowledge (when CWD is src-tauri/)
        if let Some(parent) = cwd.parent() {
            let candidate = parent.join("knowledge");
            if candidate.join(sentinel).exists() {
                log::info!("Knowledge dir found at parent/knowledge: {:?}", candidate);
                return candidate.canonicalize().unwrap_or(candidate);
            }
        }
    }

    // Try resource directory (production bundles)
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let candidate = resource_dir.join("knowledge");
        if candidate.join(sentinel).exists() {
            log::info!("Knowledge dir found at resource dir: {:?}", candidate);
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }

    // Fallback
    log::warn!(
        "Could not find knowledge directory with any strategy, using fallback \"knowledge\""
    );
    PathBuf::from("knowledge")
}

// ─── Platform-specific Stockfish Path ───

fn default_stockfish_path() -> String {
    #[cfg(target_os = "windows")]
    {
        "bin/stockfish/stockfish-windows-x86-64-avx2.exe".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "bin/stockfish/stockfish-ubuntu-x86-64-avx2".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "bin/stockfish/stockfish-macos".to_string()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        "stockfish".to_string()
    }
}

// ─── Sidecar Path Resolution ───

/// Resolve the Stockfish binary path.
///
/// First tries the sidecar binary (bundled with the app in production).
/// Falls back to the platform-specific local binary path (for development).
fn resolve_stockfish_path(app_handle: &tauri::AppHandle) -> String {
    match app_handle.shell().sidecar("stockfish") {
        Ok(_) => {
            // Sidecar was resolved successfully by Tauri.
            // Build the exact path that Tauri uses internally:
            //   {resource_dir}/binaries/{name}-{target_triple}[.exe]
            let target_triple = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
                "x86_64-pc-windows-msvc"
            } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
                "x86_64-unknown-linux-gnu"
            } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                "aarch64-apple-darwin"
            } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
                "x86_64-apple-darwin"
            } else {
                log::warn!("Unsupported platform for sidecar, falling back to local binary");
                return default_stockfish_path();
            };

            let exe_suffix = if cfg!(target_os = "windows") {
                ".exe"
            } else {
                ""
            };
            let sidecar_filename = format!("stockfish-{}{}", target_triple, exe_suffix);

            if let Ok(resource_dir) = app_handle.path().resource_dir() {
                let sidecar_path = resource_dir.join("binaries").join(&sidecar_filename);
                if sidecar_path.exists() {
                    log::info!("Using Stockfish sidecar: {:?}", sidecar_path);
                    return sidecar_path.to_string_lossy().to_string();
                }
                log::warn!(
                    "Sidecar resolved but file not found at {:?}, falling back to local binary",
                    sidecar_path
                );
            }
            default_stockfish_path()
        }
        Err(e) => {
            log::warn!("Sidecar 'stockfish' not found ({}), using local binary", e);
            default_stockfish_path()
        }
    }
}

// ─── Tauri Entry Point ───

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── Load app configuration ──
    let app_config = config::AppConfig::load().unwrap_or_default();

    // Components that don't need the app handle are created upfront.
    let inference = inference::ollama::OllamaClient::with_models(
        app_config.ollama_url.clone(),
        app_config.primary_model.clone(),
        app_config.fast_model.clone(),
        app_config.embedding_model.clone(),
    );
    log::info!(
        "LLM inference client initialized (primary: {}, fast: {}, embedding: {})",
        app_config.primary_model,
        app_config.fast_model,
        app_config.embedding_model
    );

    // Clone config values for the setup closure
    let stockfish_threads = app_config.stockfish_threads;
    let stockfish_hash_mb = app_config.stockfish_hash_mb;
    let analysis_depth = app_config.analysis_depth;
    let multipv = app_config.multipv;
    let syzygy_path = app_config.syzygy_path.clone();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("mentorfish".into()),
                    }),
                ])
                .build(),
        )
        .setup(move |app| {
            // Log log-file location for diagnostics
            if let Ok(log_dir) = app.handle().path().app_log_dir() {
                log::info!("Log directory: {:?}", log_dir);
            }

            // ── Connect to database (now that the logger is initialized) ──
            let db_config = database::DatabaseConfig::default();
            let database = {
                // Try the current Tokio handle; if not available, spawn a temporary runtime
                let result: anyhow::Result<database::DatabaseManager> =
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        handle.block_on(database::DatabaseManager::init(&db_config))
                    } else {
                        let rt = tokio::runtime::Runtime::new()
                            .expect("Failed to create Tokio runtime for DB connection");
                        rt.block_on(database::DatabaseManager::init(&db_config))
                    };
                match result {
                    Ok(db) => {
                        log::info!("Database connected successfully");
                        Some(std::sync::Arc::new(db))
                    }
                    Err(e) => {
                        log::warn!(
                            "Database connection failed: {} — running without persistence",
                            e
                        );
                        None
                    }
                }
            };

            // ── Initialize knowledge directory ──
            let knowledge_dir = resolve_knowledge_dir(app.handle());
            crate::knowledge::initialize_knowledge_dir(knowledge_dir);

            // Resolve Stockfish path: try sidecar first, fall back to local binary (or config)
            let binary_path = resolve_stockfish_path(app.handle());

            log::info!(
                "LLM inference client initialized (primary: {}, fast: {}, embedding: {})",
                inference.primary_model,
                inference.fast_model,
                inference.embedding_model,
            );

            let engine_config = engine::EngineConfig {
                binary_path: binary_path.clone(),
                threads: stockfish_threads,
                hash_mb: stockfish_hash_mb,
                depth: analysis_depth,
                multipv,
                syzygy_path,
            };
            let engine = engine::stockfish::StockfishManager::new(engine_config);
            log::info!("Stockfish engine initialized (binary: {})", binary_path);

            let app_state = std::sync::Arc::new(ipc::AppState {
                engine: Box::new(engine),
                inference: Box::new(inference),
                database: database.clone(),
            });

            app.manage(app_state);

            if database.is_some() {
                log::info!("Database connection: OK");
            } else {
                log::warn!("Database connection: unavailable — running without persistence");
            }

            Ok(())
        });

    let builder = ipc::register_commands(builder);

    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                log::info!("MentorFish shutting down...");
                let state: tauri::State<'_, std::sync::Arc<ipc::AppState>> = app_handle.state();
                let state = state.inner().clone();
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    log::info!("Shutting down Stockfish engine...");
                    let _ = state.engine.shutdown().await;
                    log::info!("Shutdown complete");
                });
            }
        });
}
