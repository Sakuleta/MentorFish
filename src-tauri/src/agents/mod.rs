// ─── Agent Specifications ───
//
// Section 7.4 of the PRD.
// Each agent is a stateless function: receives typed input, returns typed output.

use crate::{ConfidenceScore, UCIMove, FEN};
use serde::{Deserialize, Serialize};
use specta::Type;

pub mod curriculum;
pub mod executor;
pub mod memory;
pub mod pedagogical;
pub mod strategic;
pub mod tactical;
pub mod theory;

// ─── Shared Agent Types ───

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RetrievalBundle {
    pub chunks: Vec<KnowledgeChunk>,
    pub opening_node: Option<OpeningNode>,
    pub model_games: Vec<GameReference>,
}

/// A retrieved knowledge chunk from LanceDB.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct KnowledgeChunk {
    pub id: uuid::Uuid,
    pub chunk_type: ChunkType,
    pub content: String,
    pub source: String,
    pub position_fen: Option<FEN>,
    pub opening_eco: Option<String>,
    pub similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub enum ChunkType {
    Concept,
    Opening,
    Motif,
    InstructiveExample,
    EndgameTechnique,
}

/// Opening database node.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpeningNode {
    pub fen: FEN,
    pub eco: Option<String>,
    pub opening_name: Option<String>,
    pub frequency: Option<i32>,
    pub white_score: Option<f64>,
    pub children: Vec<OpeningMove>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OpeningMove {
    pub uci: UCIMove,
    pub san: String,
    pub frequency: i32,
}

/// Reference to a model game.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GameReference {
    pub white: String,
    pub black: String,
    pub year: Option<i32>,
    pub event: Option<String>,
    pub result: String,
    pub relevance: String,
}

// ─── Chat History ───

/// A single entry in a conversational chat history.
/// Mirrors the frontend `ChatHistoryEntry` type.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChatHistoryEntry {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
}

// ─── User Profile (cross-agent reference) ───

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserProfile {
    pub user_id: uuid::Uuid,
    pub tactical_accuracy: f64,
    pub positional_accuracy: f64,
    pub opening_knowledge: f64,
    pub endgame_technique: f64,
    pub time_management: f64,
    pub tilt_resistance: f64,
    pub style_profile: serde_json::Value,
    pub weakness_patterns: Vec<WeaknessPattern>,
    pub confidence: ConfidenceScore,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WeaknessPattern {
    pub id: uuid::Uuid,
    pub pattern_name: String,
    pub description: Option<String>,
    pub occurrence_count: u32,
    pub last_seen: Option<String>,
}
