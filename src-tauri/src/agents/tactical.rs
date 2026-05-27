// ─── Tactical Agent ───
//
// Section 7.4, Agent A.
// No LLM call. Pure computation over structured engine data. Always fast.

use crate::engine::EngineOutput;
use crate::features::FeatureBundle;
use crate::{ConfidenceScore, UCIMove};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalSummary {
    pub blunders: Vec<BlunderRecord>,
    pub missed_tactics: Vec<TacticRecord>,
    pub eval_swings: Vec<EvalSwing>,
    pub forcing_sequences: Vec<ForcingLine>,
    pub confidence: ConfidenceScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlunderRecord {
    pub uci_move: UCIMove,
    pub eval_swing_cp: i32,
    pub position_fen: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticRecord {
    pub uci_opportunity: UCIMove,
    pub eval_improvement_cp: i32,
    pub tactic_type: String,
    pub position_fen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSwing {
    pub move_number: u32,
    pub swing_cp: i32,
    pub from_eval: i32,
    pub to_eval: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForcingLine {
    pub uci_sequence: Vec<UCIMove>,
    pub eval_result_cp: i32,
    pub classification: String,
}

/// Compute the tactical summary from engine output and feature extraction.
/// Pure computation — no LLM call.
pub fn compute(input: TacticalInput) -> TacticalSummary {
    let engine = &input.engine_output;
    let features = &input.features;

    let mut blunders = Vec::new();
    let mut swings = Vec::new();
    let mut missed = Vec::new();

    // Identify blunders: eval swing >= 150 cp is a blunder
    if features.eval_swing_cp.abs() >= 150 {
        blunders.push(BlunderRecord {
            uci_move: String::new(), // filled by caller with actual move
            eval_swing_cp: features.eval_swing_cp,
            position_fen: features.position_fen.clone(),
            description: format!(
                "Evaluation swing of {} centipawns — {}",
                features.eval_swing_cp.abs(),
                if features.eval_swing_cp > 0 {
                    "position improved"
                } else {
                    "position worsened"
                }
            ),
        });
    }

    // Record eval swing
    swings.push(EvalSwing {
        move_number: 0, // filled by caller
        swing_cp: features.eval_swing_cp,
        from_eval: 0,
        to_eval: features.eval_cp,
    });

    // Detect missed tactics from MultiPV comparison
    for candidate in &engine.multipv {
        if candidate.multipv > 1 {
            if let (Some(cand_eval), Some(best_eval)) = (
                candidate.eval_cp,
                engine.multipv.first().and_then(|b| b.eval_cp),
            ) {
                let improvement = cand_eval - best_eval;
                if improvement > 100 {
                    missed.push(TacticRecord {
                        uci_opportunity: candidate.pv.first().cloned().unwrap_or_default(),
                        eval_improvement_cp: improvement,
                        tactic_type: "missed_candidate".to_string(),
                        position_fen: features.position_fen.clone(),
                    });
                }
            }
        }
    }

    TacticalSummary {
        blunders,
        missed_tactics: missed,
        eval_swings: swings,
        forcing_sequences: Vec::new(), // TODO: compute from engine analysis
        confidence: if features.confidence == crate::features::ExtractionConfidence::High {
            1.0
        } else {
            0.7
        },
    }
}

pub struct TacticalInput<'a> {
    pub engine_output: &'a EngineOutput,
    pub features: &'a FeatureBundle,
}
