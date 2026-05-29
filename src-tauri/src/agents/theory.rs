// ─── Theory Agent ───
//
// Section 7.4, Agent F.
// LLM call: Qwen3-14B with thinking enabled.
// Provides opening theory, model games, and transposition analysis.

use crate::ConfidenceScore;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TheoryOutput {
    pub theoretical_lines: Vec<Line>,
    pub model_games: Vec<super::GameReference>,
    pub transpositions: Vec<TranspositionNote>,
    pub novelty_flag: bool,
    pub historical_context: Option<String>,
    pub confidence: ConfidenceScore,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Line {
    pub uci_moves: Vec<String>,
    pub san_moves: Vec<String>,
    pub evaluation: Option<String>,
    pub popularity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranspositionNote {
    pub from_line: String,
    pub to_line: String,
    pub description: String,
}

pub struct TheoryInput<'a> {
    pub position_fen: &'a str,
    pub opening_node: Option<&'a super::OpeningNode>,
    pub rag_results: &'a super::RetrievalBundle,
    pub repertoire: &'a [RepertoireEntry],
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RepertoireEntry {
    pub fen: String,
    pub color: String,
    pub familiarity: f64,
}

/// Placeholder stub.
pub fn compute_stub(_input: TheoryInput) -> TheoryOutput {
    TheoryOutput {
        theoretical_lines: Vec::new(),
        model_games: Vec::new(),
        transpositions: Vec::new(),
        novelty_flag: false,
        historical_context: None,
        confidence: 0.5,
    }
}
