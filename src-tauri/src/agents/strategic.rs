// ─── Strategic Agent ───
//
// Section 7.4, Agent B.
// LLM call: Qwen3-14B with thinking enabled (post-game).
// Skipped in live coaching fast path.

use crate::ConfidenceScore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicSummary {
    pub imbalances: Vec<Imbalance>,
    pub plans: Vec<Plan>,
    pub pawn_structure: PawnStructureClassification,
    pub key_weaknesses: Vec<String>, // square names
    pub positional_themes: Vec<Theme>,
    pub confidence: ConfidenceScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imbalance {
    pub category: String, // e.g., "bishop_vs_knight", "space_vs_activity"
    pub advantage_color: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub description: String,
    pub candidate_moves: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PawnStructureClassification {
    pub structure_type: String, // e.g., "isolated_queens_pawn", "carlsbad"
    pub plans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub description: String,
}

/// Placeholder — real implementation calls Qwen3-14B via InferenceClient.
pub struct StrategicInput<'a> {
    pub features: &'a crate::features::FeatureBundle,
    pub tactical_summary: &'a super::tactical::TacticalSummary,
    pub rag_results: &'a super::RetrievalBundle,
}

pub fn compute_stub(_input: StrategicInput) -> StrategicSummary {
    StrategicSummary {
        imbalances: Vec::new(),
        plans: Vec::new(),
        pawn_structure: PawnStructureClassification {
            structure_type: "unclassified".to_string(),
            plans: Vec::new(),
        },
        key_weaknesses: Vec::new(),
        positional_themes: Vec::new(),
        confidence: 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategic_summary_default() {
        // Verify the stub returns a valid summary with expected structure
        let features = crate::features::FeatureBundle {
            position_fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
            eval_cp: 30,
            eval_swing_cp: 0,
            is_forced_mate: false,
            mate_in: None,
            top_moves: vec![],
            tactics: vec![],
            positional: vec![],
            dynamic: vec![],
            confidence: crate::features::ExtractionConfidence::High,
        };
        let tactical = crate::agents::tactical::TacticalSummary::default();
        let rag = crate::agents::RetrievalBundle {
            chunks: vec![],
            opening_node: None,
            model_games: vec![],
        };
        let input = StrategicInput {
            features: &features,
            tactical_summary: &tactical,
            rag_results: &rag,
        };
        let summary = compute_stub(input);
        assert!(summary.imbalances.is_empty());
        assert!(summary.plans.is_empty());
        assert_eq!(summary.pawn_structure.structure_type, "unclassified");
        assert!(summary.pawn_structure.plans.is_empty());
        assert!(summary.key_weaknesses.is_empty());
        assert!(summary.positional_themes.is_empty());
        assert_eq!(summary.confidence, 0.5);
    }
}
