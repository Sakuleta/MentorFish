// ─── Request / Response / Event Types ───

use serde::{Deserialize, Serialize};
use specta::Type;

// ─── Analysis Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct AnalyzePositionRequest {
    pub fen: String,
    pub depth: Option<u32>,
    pub pipeline_type: Option<String>,
    /// Optional natural-language query for RAG retrieval.
    /// If omitted, a query is auto-generated from detected features.
    pub query: Option<String>,
    /// Coaching persona override (defaults to ModernGM if omitted).
    pub persona: Option<String>,
    /// Optional game ID to link analysis moves to a saved game.
    #[serde(default)]
    pub game_id: Option<String>,
    /// Optional analyzed moves to persist when the pipeline completes.
    #[serde(default)]
    pub moves: Option<Vec<crate::Move>>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct AnalyzePositionResponse {
    pub explanation: crate::agents::pedagogical::FinalExplanation,
    pub engine_eval: i32,
    pub best_move: Option<String>,
}

/// Lightweight engine-only analysis request.
/// Does NOT run the LLM pipeline — returns raw engine output instantly.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct QuickAnalyzeRequest {
    pub fen: String,
    pub depth: Option<u32>,
}

/// Lightweight engine-only analysis response.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct QuickAnalyzeResponse {
    pub eval_cp: i32,
    pub eval_mate: Option<i32>,
    pub best_move: Option<String>,
    pub depth: u32,
    pub nodes: Option<u64>,
    pub lines: Vec<crate::engine::CandidateLine>,
}

// ─── User / Health / Error Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct UserProfileResponse {
    pub profile: crate::agents::UserProfile,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct HealthCheckResponse {
    pub engine_ok: bool,
    pub inference_ok: bool,
    pub database_ok: bool,
}

/// Error report sent from the frontend ErrorBoundary.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ErrorReport {
    pub message: String,
    pub stack: Option<String>,
    pub component: Option<String>,
    pub timestamp: String,
}

// ─── Game Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct SaveGameRequest {
    pub pgn: String,
    pub result: Option<String>,
    pub played_at: String,
    pub source: Option<String>,
    pub opening_eco: Option<String>,
    pub time_control: Option<String>,
    /// Optional analyzed moves to persist alongside the game.
    #[serde(default)]
    pub moves: Option<Vec<crate::Move>>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct SaveGameResponse {
    pub game_id: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ImportPgnRequest {
    /// Raw PGN text containing one or more games.
    pub pgn_text: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ImportPgnResponse {
    pub games_imported: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ExportPgnRequest {
    /// Optional list of game IDs to export. If omitted, all games are exported.
    pub game_ids: Option<Vec<String>>,
    /// Optional file path to write the PGN to. If omitted, returns the PGN string in the response.
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ExportPgnResponse {
    pub pgn: String,
    pub game_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct GameSummaryResponse {
    pub game_id: String,
    pub opponent: String,
    pub result: String,
    pub played_at: String,
    pub opening: String,
    pub move_count: u32,
}

// ─── Tauri Event Payloads ───

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct StreamingTokenEvent {
    pub token: String,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CoachingAlertEvent {
    pub alert_type: String,
    pub message: String,
    pub position_fen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EngineProgressEvent {
    pub depth: u32,
    pub eval_cp: i32,
    pub best_move: Option<String>,
    pub nodes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CoachingTriggerEvent {
    pub trigger_type: String,
    pub message: String,
    pub severity: String,
    pub position_fen: String,
}

// ─── Chat Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ChatMessageRequest {
    pub message: String,
    pub fen: Option<String>,
    pub history: Vec<crate::agents::ChatHistoryEntry>,
    pub persona: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ChatMessageResponse {
    pub reply: String,
}

// ─── Play Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MakeMoveRequest {
    pub fen: String,
    pub uci: String,
    pub vs_ai: bool,
    /// Strength mode: "full" (default), "stockfish_elo", "boltzmann", or "training"
    #[serde(default)]
    pub strength_mode: Option<String>,
    /// Target ELO for strength-limited modes (1320..3190 for stockfish_elo)
    #[serde(default)]
    pub target_elo: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MakeMoveResponse {
    pub fen: String,
    pub is_check: bool,
    pub is_checkmate: bool,
    pub is_stalemate: bool,
    pub ai_move: Option<String>,
    pub ai_fen: Option<String>,
}

// ─── Collapse Detection Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CollapseDetectRequest {
    /// Move history: each entry is (uci, eval_swing_cp, move_time_ms)
    pub moves: Vec<(String, i32, Option<u64>)>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct CollapseDetectResponse {
    pub collapses: Vec<crate::orchestrator::CollapseEvent>,
}

// ─── Opening Explorer Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct OpeningNodeResponse {
    pub node: Option<crate::agents::OpeningNode>,
}

// ─── Knowledge Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct IngestionReportResponse {
    pub books_processed: u64,
    pub chunks_created: u64,
    pub chunks_embedded: u64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct KnowledgeSummary {
    pub total_books: u64,
    pub total_chunks: u64,
    pub total_embedded: u64,
    pub books: Vec<BookSummary>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct BookSummary {
    pub title: String,
    pub chunk_count: u64,
    pub chunk_type: String,
    pub has_embeddings: bool,
}

#[derive(Debug, Deserialize, Type)]
pub struct CopyToKnowledgeRequest {
    pub file_name: String,
    pub file_content: Vec<u8>,
    pub file_type: String,
}

// ─── Book Reader Types ───

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct GetBookChunksRequest {
    /// Book title / source name to filter chunks by.
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct GetBookChunksResponse {
    pub chunks: Vec<crate::agents::KnowledgeChunk>,
}
