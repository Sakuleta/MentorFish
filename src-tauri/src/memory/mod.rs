// ─── Memory System ───
//
// Section 10 of the PRD.
// Persists and updates the user profile across sessions.

use crate::agents::memory::{confidence_score, ProfileDelta};
use crate::agents::UserProfile;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// ─── PGN Parsing (Section 8.2) ───

/// A parsed PGN game with its tag-pair headers and raw movetext.
#[derive(Debug, Clone)]
pub struct ParsedGame {
    pub headers: Vec<(String, String)>,
    pub movetext: String,
}

/// Cached regex patterns for PGN parsing (compiled once, reused across calls).
fn tag_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"\[(\w+)\s+"([^"]*)"\]"#).unwrap())
}

fn boundary_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\n\s*\n(\[Event\s)").unwrap())
}

fn game_split_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\n___GAME_SPLIT___\n").unwrap())
}

/// Parse raw PGN text (single or multiple games) into structured `ParsedGame` records.
///
/// Uses regex-based tag-pair extraction. Handles multiple games by splitting on
/// blank-line boundaries that precede `[Event` headers.
pub fn parse_pgn_games(pgn_text: &str) -> Vec<ParsedGame> {
    let tagged = boundary_re().replace_all(pgn_text, "\n___GAME_SPLIT___\n$1");
    let chunks: Vec<&str> = game_split_re().split(&tagged).collect();
    let mut games: Vec<ParsedGame> = Vec::new();

    for chunk in &chunks {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut headers: Vec<(String, String)> = Vec::new();
        let mut last_match_end: usize = 0;

        for cap in tag_re().captures_iter(trimmed) {
            let key = cap[1].to_string();
            let value = cap[2].to_string();
            headers.push((key, value));
            last_match_end = cap.get(0).unwrap().end();
        }

        // A game must have at least one header tag
        if headers.is_empty() {
            continue;
        }

        let movetext = trimmed[last_match_end..].trim().to_string();
        games.push(ParsedGame { headers, movetext });
    }

    games
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub game_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub pgn: String,
    pub result: Option<String>,
    pub played_at: String,
    pub source: String,
    pub opening_eco: Option<String>,
    pub time_control: Option<String>,
}

/// Apply a ProfileDelta to the current UserProfile and return the updated profile.
///
/// Uses EMA (Exponential Moving Average) for dimension updates:
///   new_value = 0.15 * game_value + 0.85 * current_value
///
/// For weakness patterns:
///   - New patterns are added with occurrence_count = 1
///   - Existing patterns have their occurrence_count incremented and last_seen updated
///
/// Confidence is computed from the total sample count across all dimensions.
pub fn apply_delta(profile: &UserProfile, delta: &ProfileDelta) -> UserProfile {
    let mut updated = profile.clone();

    // ── Apply EMA updates to each dimension ──
    for update in &delta.dimension_updates {
        let current = match update.dimension {
            crate::agents::memory::ProfileDimension::TacticalAccuracy => {
                updated.tactical_accuracy
            }
            crate::agents::memory::ProfileDimension::PositionalAccuracy => {
                updated.positional_accuracy
            }
            crate::agents::memory::ProfileDimension::OpeningKnowledge => {
                updated.opening_knowledge
            }
            crate::agents::memory::ProfileDimension::EndgameTechnique => {
                updated.endgame_technique
            }
            crate::agents::memory::ProfileDimension::TimeManagement => {
                updated.time_management
            }
            crate::agents::memory::ProfileDimension::TiltResistance => {
                updated.tilt_resistance
            }
        };

        // EMA: new_value = 0.15 * game_value + 0.85 * current_value
        let ema_value = crate::agents::memory::ema_update(current, update.game_value);

        match update.dimension {
            crate::agents::memory::ProfileDimension::TacticalAccuracy => {
                updated.tactical_accuracy = ema_value;
            }
            crate::agents::memory::ProfileDimension::PositionalAccuracy => {
                updated.positional_accuracy = ema_value;
            }
            crate::agents::memory::ProfileDimension::OpeningKnowledge => {
                updated.opening_knowledge = ema_value;
            }
            crate::agents::memory::ProfileDimension::EndgameTechnique => {
                updated.endgame_technique = ema_value;
            }
            crate::agents::memory::ProfileDimension::TimeManagement => {
                updated.time_management = ema_value;
            }
            crate::agents::memory::ProfileDimension::TiltResistance => {
                updated.tilt_resistance = ema_value;
            }
        }
    }

    // ── Update confidence from total sample counts ──
    let total_samples: u32 = delta
        .dimension_updates
        .iter()
        .map(|u| u.sample_count)
        .sum();
    // Blend existing confidence with new samples
    let new_confidence = confidence_score(total_samples);
    updated.confidence = (updated.confidence * 0.7 + new_confidence * 0.3).min(1.0);

    // ── Merge weakness patterns ──
    for flag in &delta.new_weakness_flags {
        if let Some(existing) = updated
            .weakness_patterns
            .iter_mut()
            .find(|w| w.pattern_name == flag.pattern_name)
        {
            // Increment occurrence count and update last_seen
            existing.occurrence_count += 1;
            existing.last_seen = Some(chrono::Utc::now().to_rfc3339());
            // Update description if a new example FEN is provided
            if let Some(ref example_fen) = flag.example_fen {
                if existing.description.is_none() {
                    existing.description = Some(flag.description.clone());
                }
                // Log the example FEN for future reference
                log::debug!(
                    "Weakness '{}' occurrence #{} at {}",
                    existing.pattern_name,
                    existing.occurrence_count,
                    example_fen
                );
            }
        } else {
            // New weakness pattern
            updated
                .weakness_patterns
                .push(crate::agents::WeaknessPattern {
                    id: uuid::Uuid::new_v4(),
                    pattern_name: flag.pattern_name.clone(),
                    description: Some(flag.description.clone()),
                    occurrence_count: 1,
                    last_seen: Some(chrono::Utc::now().to_rfc3339()),
                });
        }
    }

    // ── Prune weakness patterns with low occurrence (cleanup) ──
    // Remove patterns seen only once that are older than 30 days
    let now = chrono::Utc::now();
    updated.weakness_patterns.retain(|w| {
        if w.occurrence_count <= 1 {
            if let Some(ref last_seen) = w.last_seen {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_seen) {
                    let age = now.signed_duration_since(dt);
                    return age.num_days() < 30;
                }
            }
        }
        true // Keep patterns with multiple occurrences
    });

    updated
}

// ─── Weakness Pattern Clustering (Section 10.3) ───

/// Stub for the offline clustering job that runs every 5 games.
pub fn should_run_clustering(game_count: u32) -> bool {
    game_count > 0 && game_count % 5 == 0
}
