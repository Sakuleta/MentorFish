// ─── Engine Layer ───
//
// Section 5 of the PRD.
// Manages the Stockfish child process via UCI protocol.

pub mod play;
pub mod stockfish;

use crate::{UCIMove, FEN};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Output from a Stockfish analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EngineOutput {
    pub fen: FEN,
    pub eval_cp: i32,
    pub eval_mate: Option<i32>,
    pub best_move: Option<UCIMove>,
    pub best_move_san: Option<String>,
    pub ponder: Option<UCIMove>,
    pub depth: u32,
    pub multipv: Vec<CandidateLine>,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
    pub time_ms: Option<u64>,
}

impl Default for EngineOutput {
    fn default() -> Self {
        Self {
            fen: "8/8/8/8/8/8/8/8 w - - 0 1".into(),
            eval_cp: 0,
            eval_mate: None,
            best_move: None,
            best_move_san: None,
            ponder: None,
            depth: 0,
            multipv: vec![],
            nodes: None,
            nps: None,
            time_ms: None,
        }
    }
}

/// A single line from MultiPV analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CandidateLine {
    pub multipv: u32,
    pub pv: Vec<UCIMove>,
    pub eval_cp: Option<i32>,
    pub eval_mate: Option<i32>,
    pub depth: u32,
}

/// Configuration for the Stockfish engine.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EngineConfig {
    /// Path to Stockfish binary
    pub binary_path: String,
    /// Number of CPU threads
    pub threads: u32,
    /// Hash table size in MB
    pub hash_mb: u32,
    /// Number of principal variations
    pub multipv: u32,
    /// Analysis depth
    pub depth: u32,
    /// Path to Syzygy tablebases (optional)
    pub syzygy_path: Option<String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            binary_path: "stockfish".to_string(),
            threads: num_cpus::get() as u32 - 2,
            hash_mb: 2048,
            multipv: 5,
            depth: 22,
            syzygy_path: None,
        }
    }
}

// ─── Engine Manager Trait ───

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Callback invoked during Stockfish analysis for each `info depth` line.
/// Parameters: (depth, eval_cp, nodes)
pub type EngineProgressFn = Arc<dyn Fn(u32, i32, Option<u64>) + Send + Sync>;

#[async_trait]
pub trait EngineManager: Send + Sync {
    /// Initialize the engine with the given configuration.
    async fn configure(&self, config: EngineConfig) -> Result<()>;

    /// Analyze a position and return structured output.
    ///
    /// If `on_progress` is provided, it is called for each `info depth` line
    /// with the current depth, evaluation (centipawns), and node count.
    async fn analyze(
        &self,
        fen: &FEN,
        depth: Option<u32>,
        on_progress: Option<EngineProgressFn>,
    ) -> Result<EngineOutput>;

    /// Get the best move for a position (play mode).
    async fn best_move(&self, fen: &FEN, depth: Option<u32>) -> Result<UCIMove>;

    /// Check if the engine is healthy and responsive.
    async fn health_check(&self) -> Result<bool>;

    /// Configure Stockfish strength limiting for ELO-scaled play.
    /// - `Some(elo)`: enable UCI_LimitStrength and set UCI_Elo
    /// - `None`: disable UCI_LimitStrength (back to full strength)
    async fn configure_strength(&self, elo: Option<u32>) -> Result<()>;

    /// Set an arbitrary UCI option on the engine.
    async fn set_uci_option(&self, name: &str, value: &str) -> Result<()>;

    /// Shut down the engine process.
    async fn shutdown(&self) -> Result<()>;
}
