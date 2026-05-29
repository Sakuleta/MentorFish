// ─── IPC Layer (Tauri Commands) ───
//
// Section 11.2 of the PRD.
// All frontend-to-backend communication goes through these commands.

pub mod analysis;
pub mod chat;
pub mod config;
pub mod games;
pub mod helpers;
pub mod knowledge;
pub mod play;
pub mod profile;
pub mod types;

pub use helpers::*;
pub use types::*;

use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::engine::EngineManager;
use crate::inference::InferenceClient;

// ─── Application State ───

/// Shared application state, managed by Tauri.
pub struct AppState {
    pub engine: Box<dyn EngineManager>,
    pub inference: Box<dyn InferenceClient>,
    pub database: Option<Arc<DatabaseManager>>,
    /// Semaphore limiting concurrent Stockfish analyses to prevent resource exhaustion.
    /// Max 2 concurrent analyses — additional requests queue behind it.
    pub analysis_semaphore: tokio::sync::Semaphore,
}

impl AppState {
    pub fn new(
        engine: Box<dyn EngineManager>,
        inference: Box<dyn InferenceClient>,
        database: Option<Arc<DatabaseManager>>,
    ) -> Self {
        Self {
            engine,
            inference,
            database,
            analysis_semaphore: tokio::sync::Semaphore::new(2),
        }
    }
}

// ─── Tauri Command Registrations ───

pub fn register_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        analysis::cmd_analyze_position,
        analysis::cmd_quick_analyze,
        analysis::cmd_stream_analyze,
        analysis::cmd_detect_collapse,
        chat::cmd_chat_message,
        chat::cmd_chat_message_stream,
        profile::cmd_get_user_profile,
        profile::cmd_generate_curriculum,
        config::cmd_health_check,
        config::cmd_get_config,
        config::cmd_save_config,
        config::cmd_report_error,
        knowledge::cmd_run_ingestion,
        knowledge::cmd_get_knowledge_summary,
        knowledge::cmd_copy_to_knowledge,
        knowledge::cmd_get_book_chunks,
        play::cmd_make_move,
        play::cmd_ai_move,
        play::cmd_get_legal_moves,
        games::cmd_save_game,
        games::cmd_get_game_moves,
        games::cmd_import_pgn,
        games::cmd_export_pgn,
        games::cmd_get_recent_games,
        games::cmd_get_opening,
    ])
}
