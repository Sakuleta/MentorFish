// ─── Memory Agent ───
//
// Section 7.4, Agent D and Section 10.
// No LLM call. Pure computation. Updates user profile after each game.

use crate::ConfidenceScore;
use serde::{Deserialize, Serialize};

/// Delta to apply to the user profile after a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDelta {
    pub dimension_updates: Vec<DimensionUpdate>,
    pub new_weakness_flags: Vec<WeaknessFlag>,
    pub opening_repertoire_events: Vec<RepertoireEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionUpdate {
    pub dimension: ProfileDimension,
    pub game_value: f64,
    pub new_value: f64,
    pub sample_count: u32,
    pub confidence: ConfidenceScore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProfileDimension {
    TacticalAccuracy,
    PositionalAccuracy,
    OpeningKnowledge,
    EndgameTechnique,
    TimeManagement,
    TiltResistance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaknessFlag {
    pub pattern_name: String,
    pub description: String,
    pub example_fen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepertoireEvent {
    pub fen: String,
    pub color: String,
    pub event_type: RepertoireEventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepertoireEventType {
    Played,
    Deviated,
    NewLine,
}

// ─── EMA Update Algorithm (Section 10.2) ───

/// Exponential Moving Average update.
/// `new_value = 0.15 * game_value + 0.85 * current_value`
pub fn ema_update(current: f64, game_value: f64) -> f64 {
    0.15 * game_value + 0.85 * current
}

/// Confidence score: `min(1.0, sample_count / 20.0)`
pub fn confidence_score(sample_count: u32) -> f64 {
    (sample_count as f64 / 20.0).min(1.0)
}

/// Decay function for stale dimensions (90+ days without update).
/// `decay_per_day = (current_value - 0.5) * 0.005`
pub fn daily_decay(current: f64) -> f64 {
    (current - 0.5) * 0.005
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_update() {
        let result = ema_update(0.8, 0.4);
        // 0.15 * 0.4 + 0.85 * 0.8 = 0.06 + 0.68 = 0.74
        assert!((result - 0.74).abs() < 0.001);
    }

    #[test]
    fn test_confidence_score() {
        assert_eq!(confidence_score(0), 0.0);
        assert!((confidence_score(10) - 0.5).abs() < 0.001);
        assert_eq!(confidence_score(20), 1.0);
        assert_eq!(confidence_score(50), 1.0);
    }
}
