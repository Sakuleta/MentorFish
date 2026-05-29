// ─── Shared Helpers ───

use serde::Serialize;
use tauri::Emitter;

use crate::database::{DatabaseManager, ProfileStore};

/// Parse a persona string (from the frontend) into the `Persona` enum.
/// Falls back to `ModernGM` if unrecognized or missing.
pub fn parse_persona(raw: &Option<String>) -> crate::agents::pedagogical::Persona {
    use crate::agents::pedagogical::Persona;
    match raw.as_deref() {
        Some("soviet" | "soviet_coach" | "sovietCoach") => Persona::SovietCoach,
        Some("modern" | "modern_gm" | "modernGm" | "modernGM") => Persona::ModernGM,
        Some("calm" | "calm_teacher" | "calmTeacher") => Persona::CalmTeacher,
        Some("brutal" | "brutal_analyst" | "brutalAnalyst") => Persona::BrutalAnalyst,
        Some("psych" | "psychological" | "psychologicalMentor") => Persona::PsychologicalMentor,
        _ => Persona::ModernGM,
    }
}

/// Parse a pipeline_type string (from the frontend) into the `PipelineType` enum.
/// Falls back to `PostGame` if unrecognized or missing.
pub fn parse_pipeline_type(raw: &Option<String>) -> crate::orchestrator::PipelineType {
    match raw.as_deref() {
        Some("postGame" | "post_game" | "postgame" | "post-game" | "review") => {
            crate::orchestrator::PipelineType::PostGame
        }
        Some("liveCoaching" | "live_coaching" | "livecoaching" | "live-coaching" | "live") => {
            crate::orchestrator::PipelineType::LiveCoaching
        }
        Some("theory" | "opening" | "opening_theory") => crate::orchestrator::PipelineType::Theory,
        Some("curriculum" | "study" | "study_plan") => {
            crate::orchestrator::PipelineType::Curriculum
        }
        Some("conversational" | "chat" | "conversation") => {
            crate::orchestrator::PipelineType::Conversational
        }
        _ => {
            if raw.is_some() {
                log::warn!(
                    "Unrecognized pipeline_type '{}', falling back to PostGame",
                    raw.as_deref().unwrap_or("")
                );
            }
            crate::orchestrator::PipelineType::PostGame
        }
    }
}

/// Helper to emit a Tauri event, logging a warning if the emit fails.
pub fn emit_event<T: Serialize + Clone>(window: &tauri::Window, event: &str, payload: T) {
    if let Err(e) = window.emit(event, payload) {
        log::warn!("Failed to emit '{}' event: {}", event, e);
    }
}

/// Build a default RAG query string from detected positional and tactical features.
pub fn build_default_query(features: &crate::features::FeatureBundle) -> String {
    let mut parts: Vec<String> = Vec::new();

    for t in &features.tactics {
        parts.push(t.query_term().into());
    }
    for p in &features.positional {
        parts.push(p.query_term().into());
    }
    for d in &features.dynamic {
        parts.push(d.query_term().into());
    }

    // If nothing specific was detected, use a broad query based on game phase
    if parts.is_empty() {
        let phase = detect_game_phase(features);
        match phase {
            "opening" => parts.push("opening principles development".into()),
            "endgame" => parts.push("endgame technique pawn promotion".into()),
            _ => parts.push("middlegame strategy tactics".into()),
        }
    }

    // Deduplicate and limit
    parts.sort();
    parts.dedup();
    parts.truncate(5);

    parts.join(" ")
}

/// Heuristic game-phase detection based on piece counts in the FEN.
pub fn detect_game_phase(features: &crate::features::FeatureBundle) -> &'static str {
    let fen = &features.position_fen;
    let board_part = fen.split_whitespace().next().unwrap_or("");
    // Count all pieces (both colors, all types) for accurate phase detection
    let total_pieces: usize = board_part.chars().filter(|c| c.is_alphabetic()).count();

    if total_pieces >= 28 {
        "opening"
    } else if total_pieces <= 10 {
        "endgame"
    } else {
        "middlegame"
    }
}

/// Load the user profile from the database, or create a new one.
/// Falls back to a default profile when no database is available.
pub async fn load_or_create_profile(db: Option<&DatabaseManager>) -> crate::agents::UserProfile {
    if let Some(db) = db {
        // Try to get the default user ID and their profile
        if let Ok(user_id) = db.default_user_id().await {
            if let Ok(Some(profile)) = db.get_profile(user_id).await {
                return profile;
            }
            // Profile doesn't exist yet — create and save one
            let new_profile = crate::agents::UserProfile {
                user_id,
                tactical_accuracy: 0.5,
                positional_accuracy: 0.5,
                opening_knowledge: 0.5,
                endgame_technique: 0.5,
                time_management: 0.5,
                tilt_resistance: 0.5,
                style_profile: serde_json::json!({}),
                weakness_patterns: vec![],
                confidence: 0.0,
            };
            if let Err(e) = db.save_profile(&new_profile).await {
                log::warn!("Failed to save new user profile: {}", e);
            }
            return new_profile;
        }
    }

    // No database available — return a transient default profile
    crate::agents::UserProfile {
        user_id: uuid::Uuid::new_v4(),
        tactical_accuracy: 0.5,
        positional_accuracy: 0.5,
        opening_knowledge: 0.5,
        endgame_technique: 0.5,
        time_management: 0.5,
        tilt_resistance: 0.5,
        style_profile: serde_json::json!({}),
        weakness_patterns: vec![],
        confidence: 0.0,
    }
}

/// Parse strength fields from the frontend request into a PlayStrength variant.
/// Backward-compatible: returns FullStrength if no mode is specified.
pub fn parse_strength(
    mode: &Option<String>,
    target_elo: Option<u32>,
) -> crate::engine::play::PlayStrength {
    match mode.as_deref() {
        Some("stockfish_elo") => {
            crate::engine::play::PlayStrength::StockfishElo(target_elo.unwrap_or(2000))
        }
        Some("boltzmann") => crate::engine::play::PlayStrength::Boltzmann {
            target_elo: target_elo.unwrap_or(2000),
        },
        Some("training") => crate::engine::play::PlayStrength::Training,
        _ => crate::engine::play::PlayStrength::FullStrength,
    }
}
