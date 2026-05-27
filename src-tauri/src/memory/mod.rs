// ─── Memory System ───
//
// Section 10 of the PRD.
// Persists and updates the user profile across sessions.

use crate::agents::memory::{confidence_score, ProfileDelta};
use crate::agents::UserProfile;
use serde::{Deserialize, Serialize};

// ─── PGN Parsing (Section 8.2) ───

/// A parsed PGN game with its tag-pair headers and raw movetext.
#[derive(Debug, Clone)]
pub struct ParsedGame {
    pub headers: Vec<(String, String)>,
    pub movetext: String,
}

/// Parse raw PGN text (single or multiple games) into structured `ParsedGame` records.
///
/// Uses regex-based tag-pair extraction. Handles multiple games by splitting on
/// blank-line boundaries that precede `[Event` headers.
pub fn parse_pgn_games(pgn_text: &str) -> Vec<ParsedGame> {
    let tag_re = regex::Regex::new(r#"\[(\w+)\s+"([^"]*)"\]"#).unwrap();
    // Split on blank lines immediately before [Event tags
    // Uses a two-step approach since the `regex` crate doesn't support lookahead:
    // 1. Replace blank-line + [Event boundary with a delimiter marker
    // 2. Split on that marker
    // Captures `[Event ` so it's preserved in the replacement.
    let boundary_re = regex::Regex::new(r"\n\s*\n(\[Event\s)").unwrap();
    let tagged = boundary_re.replace_all(pgn_text, "\n___GAME_SPLIT___\n$1");
    let game_split_re = regex::Regex::new(r"\n___GAME_SPLIT___\n").unwrap();

    let chunks: Vec<&str> = game_split_re.split(&tagged).collect();
    let mut games: Vec<ParsedGame> = Vec::new();

    for chunk in &chunks {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut headers: Vec<(String, String)> = Vec::new();
        let mut last_match_end: usize = 0;

        for cap in tag_re.captures_iter(trimmed) {
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
pub fn apply_delta(profile: &UserProfile, delta: &ProfileDelta) -> UserProfile {
    let mut updated = profile.clone();

    for update in &delta.dimension_updates {
        match update.dimension {
            crate::agents::memory::ProfileDimension::TacticalAccuracy => {
                updated.tactical_accuracy = update.new_value;
            }
            crate::agents::memory::ProfileDimension::PositionalAccuracy => {
                updated.positional_accuracy = update.new_value;
            }
            crate::agents::memory::ProfileDimension::OpeningKnowledge => {
                updated.opening_knowledge = update.new_value;
            }
            crate::agents::memory::ProfileDimension::EndgameTechnique => {
                updated.endgame_technique = update.new_value;
            }
            crate::agents::memory::ProfileDimension::TimeManagement => {
                updated.time_management = update.new_value;
            }
            crate::agents::memory::ProfileDimension::TiltResistance => {
                updated.tilt_resistance = update.new_value;
            }
        }
    }

    // Update confidence based on sample counts
    let min_samples = delta
        .dimension_updates
        .iter()
        .map(|u| u.sample_count)
        .min()
        .unwrap_or(0);
    updated.confidence = confidence_score(min_samples);

    // Merge new weakness patterns
    for flag in &delta.new_weakness_flags {
        if !updated
            .weakness_patterns
            .iter()
            .any(|w| w.pattern_name == flag.pattern_name)
        {
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

    updated
}

// ─── Weakness Pattern Clustering (Section 10.3) ───

/// Stub for the offline clustering job that runs every 5 games.
pub fn should_run_clustering(game_count: u32) -> bool {
    game_count > 0 && game_count % 5 == 0
}
