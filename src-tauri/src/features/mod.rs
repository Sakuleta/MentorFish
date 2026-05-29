// ─── Feature Extraction Layer ───
//
// Section 9 of the PRD.
// Step 1: Rule-based extraction (shakmaty)
// Step 2: Stockfish UCI output parsing

pub mod extractor;

use crate::{UCIMove, FEN};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Complete extracted feature set for a position.
/// LLM never sees raw FEN or raw engine output — only this struct.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FeatureBundle {
    pub position_fen: FEN,
    pub eval_cp: i32,
    pub eval_swing_cp: i32,
    pub is_forced_mate: bool,
    pub mate_in: Option<i32>,
    pub top_moves: Vec<CandidateMove>,
    pub tactics: Vec<TacticalFeature>,
    pub positional: Vec<PositionalFeature>,
    pub dynamic: Vec<DynamicFeature>,
    pub confidence: ExtractionConfidence,
}

/// A candidate move from MultiPV analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CandidateMove {
    pub uci: UCIMove,
    pub san: Option<String>,
    pub eval_cp: Option<i32>,
    pub mate_in: Option<i32>,
    pub eval_loss_cp: Option<i32>,
    pub pv: Vec<UCIMove>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
pub enum ExtractionConfidence {
    /// All extraction steps completed without errors.
    High,
    /// Any step produced incomplete output.
    Medium,
}

// ─── Tactical Features (Step 1: rule-based) ───

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum TacticalFeature {
    Fork {
        attacker_square: String,
        target_squares: Vec<String>,
    },
    Pin {
        pinned_piece_square: String,
        pinner_square: String,
        shielded_piece_square: String,
        pin_type: PinType,
    },
    Skewer {
        skewered_piece_square: String,
        attacker_square: String,
        shielded_piece_square: String,
    },
    HangingPiece {
        square: String,
        piece_type: String,
    },
    DiscoveredAttack {
        mover_square: String,
        revealed_attacker_square: String,
        target_square: String,
    },
}

impl TacticalFeature {
    pub fn query_term(&self) -> &'static str {
        match self {
            TacticalFeature::Fork { .. } => "fork",
            TacticalFeature::Pin { .. } => "pin",
            TacticalFeature::Skewer { .. } => "skewer",
            TacticalFeature::HangingPiece { .. } => "hanging piece",
            TacticalFeature::DiscoveredAttack { .. } => "discovered attack",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum PinType {
    Absolute, // King behind pinned piece
    Relative, // More valuable piece behind pinned piece
}

// ─── Positional Features (Step 1: rule-based) ───

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum PositionalFeature {
    IsolatedPawn {
        square: String,
        color: String,
    },
    DoubledPawn {
        file: String,
        color: String,
    },
    BackwardPawn {
        square: String,
        color: String,
    },
    PassedPawn {
        square: String,
        color: String,
    },
    Outpost {
        square: String,
        color: String,
    },
    OpenFile {
        file: String,
    },
    HalfOpenFile {
        file: String,
        color: String,
    },
    BishopPair {
        color: String,
    },
    PawnIsland {
        color: String,
        count: u32,
    },
    KingSafety {
        color: String,
        pawn_shield_completeness: f64,
        open_files_near_king: u32,
    },
}

impl PositionalFeature {
    pub fn query_term(&self) -> &'static str {
        match self {
            PositionalFeature::IsolatedPawn { .. } => "isolated pawn",
            PositionalFeature::DoubledPawn { .. } => "doubled pawns",
            PositionalFeature::BackwardPawn { .. } => "backward pawn",
            PositionalFeature::PassedPawn { .. } => "passed pawn",
            PositionalFeature::Outpost { .. } => "outpost square",
            PositionalFeature::OpenFile { .. } => "open file",
            PositionalFeature::HalfOpenFile { .. } => "half open file",
            PositionalFeature::BishopPair { .. } => "bishop pair",
            PositionalFeature::PawnIsland { .. } => "pawn structure",
            PositionalFeature::KingSafety { .. } => "king safety",
        }
    }
}

// ─── Dynamic Features (Step 1: rule-based) ───

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum DynamicFeature {
    PieceMobility {
        square: String,
        legal_move_count: u32,
    },
    SpaceAdvantage {
        color: String,
        controlled_squares: u32,
    },
    Development {
        color: String,
        minor_pieces_developed: u32,
    },
    Initiative {
        color: String,
        threats_count: u32,
    },
}

impl DynamicFeature {
    pub fn query_term(&self) -> &'static str {
        match self {
            DynamicFeature::PieceMobility { .. } => "piece mobility",
            DynamicFeature::SpaceAdvantage { .. } => "space advantage",
            DynamicFeature::Development { .. } => "development",
            DynamicFeature::Initiative { .. } => "initiative",
        }
    }
}
