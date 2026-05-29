// ─── Orchestrator ───
//
// Section 7 of the PRD.
// In-process stateless pipeline. Routes pipelines by context type,
// executes agent chains in defined order, manages model loading strategy.

pub mod engine;

use crate::agents::pedagogical::Persona;
use crate::agents::{RetrievalBundle, UserProfile};
use crate::engine::EngineOutput;
use crate::features::{DynamicFeature, FeatureBundle};
use crate::{Move, FEN};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;

// ─── Live Coaching Triggers (PRD Section 3.4) ───

/// A coaching event triggered during live game play.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LiveCoachingEvent {
    pub trigger_type: String, // "blunder", "weakness_match", "theory_departure", "user_request"
    pub message: String,
    pub severity: String, // "info", "warning", "critical"
    pub position_fen: String,
}

/// Evaluate the current position against live coaching triggers.
///
/// Checks five trigger categories:
/// 1. Blunder alert — eval swing >= 150cp on the user's move
/// 2. Weakness pattern match — position matches a known weakness from the user profile
/// 3. Opening theory departure — move diverges from known theory past move 6
/// 4. Time pressure — remaining time < 20% of total AND position complexity is high
/// 5. User request — placeholder; explicit questions are handled by the conversational pipeline
#[allow(clippy::too_many_arguments)]
pub fn check_coaching_triggers(
    fen: &str,
    prev_eval_cp: Option<i32>,
    engine_output: &EngineOutput,
    features: &FeatureBundle,
    user_profile: &UserProfile,
    move_number: u32,
    remaining_time_ms: Option<u64>,
    total_time_ms: Option<u64>,
) -> Vec<LiveCoachingEvent> {
    let mut events = Vec::new();

    // 1. Blunder detection: eval swing >= 150cp
    if let Some(prev) = prev_eval_cp {
        let swing = (engine_output.eval_cp - prev).abs();
        if swing >= 150 {
            events.push(LiveCoachingEvent {
                trigger_type: "blunder".into(),
                message: format!(
                    "Blunder detected! Evaluation changed by {} centipawns.",
                    swing / 100
                ),
                severity: "critical".into(),
                position_fen: fen.to_string(),
            });
        }
    }

    // 2. Weakness pattern match: check if position matches any known weakness
    for pattern in &user_profile.weakness_patterns {
        if pattern.occurrence_count >= 3 {
            // Simple check: if the feature bundle contains patterns similar to weakness
            let has_tactical = features.tactics.iter().any(|t| {
                format!("{:?}", t)
                    .to_lowercase()
                    .contains(&pattern.pattern_name.to_lowercase())
            });
            if has_tactical {
                events.push(LiveCoachingEvent {
                    trigger_type: "weakness_match".into(),
                    message: format!(
                        "This position relates to your known weakness: {}",
                        pattern.pattern_name
                    ),
                    severity: "warning".into(),
                    position_fen: fen.to_string(),
                });
            }
        }
    }

    // 3. Opening departure: if move_number > 6, could trigger theory check
    if move_number > 6 && features.eval_swing_cp.abs() > 100 {
        events.push(LiveCoachingEvent {
            trigger_type: "theory_departure".into(),
            message: "You've left known theory. Proceed carefully.".into(),
            severity: "info".into(),
            position_fen: fen.to_string(),
        });
    }

    // 4. Time pressure: remaining time < 20% of total AND complex position
    if let (Some(remaining), Some(total)) = (remaining_time_ms, total_time_ms) {
        if total > 0 && (remaining as f64 / total as f64) < 0.20 {
            // Check position complexity: more than 30 legal moves for any piece
            let complexity = features.dynamic.iter().any(|d| {
                matches!(d, DynamicFeature::PieceMobility { legal_move_count, .. } if *legal_move_count > 30)
            });
            if complexity {
                events.push(LiveCoachingEvent {
                    trigger_type: "time_pressure".into(),
                    message: "You're low on time in a complex position. Consider simplifying."
                        .into(),
                    severity: "warning".into(),
                    position_fen: fen.to_string(),
                });
            }
        }
    }

    events
}

// ─── Psychological Collapse Detection (PRD Section 3.4) ───

/// A detected psychological collapse event in post-game analysis.
/// Three or more consecutive user moves with total eval drop >= 300cp.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CollapseEvent {
    pub start_move: String,
    pub end_move: String,
    pub total_eval_drop: i32,
    pub consecutive_moves: u32,
}

/// Detect psychological collapse: sequences of 3+ consecutive moves whose
/// cumulative absolute eval swing meets or exceeds 300 centipawns.
///
/// Scans move history using sliding windows of 3 moves. Adjacent windows
/// that all exceed the threshold are merged into a single collapse event.
pub fn detect_collapse(moves: &[(String, i32, Option<u64>)]) -> Vec<CollapseEvent> {
    // moves: Vec<(uci, eval_swing_cp, move_time_ms)>
    let mut events = Vec::new();
    let n = moves.len();
    if n < 3 {
        return events;
    }

    let mut streak = 0u32;
    let mut total_drop = 0i32;
    let mut streak_start = 0usize;

    let windows: Vec<(usize, i32)> = moves
        .windows(3)
        .enumerate()
        .map(|(i, w)| {
            let sum: i32 = w.iter().map(|(_, swing, _)| swing.abs()).sum();
            (i, sum)
        })
        .collect();

    for (window_idx, drop_sum) in &windows {
        if *drop_sum >= 300 {
            if streak == 0 {
                streak_start = *window_idx;
            }
            streak += 1;
            total_drop += *drop_sum;
        } else {
            if streak >= 1 {
                let end_idx = (streak_start + streak as usize + 1).min(n - 1);
                events.push(CollapseEvent {
                    start_move: moves[streak_start].0.clone(),
                    end_move: moves[end_idx].0.clone(),
                    total_eval_drop: total_drop,
                    consecutive_moves: streak + 2, // 3 moves per window, so N windows = N+2 moves
                });
            }
            streak = 0;
            total_drop = 0;
        }
    }

    // Emit any trailing collapse that reaches the end
    if streak >= 1 {
        let end_idx = (streak_start + streak as usize + 1).min(n - 1);
        events.push(CollapseEvent {
            start_move: moves[streak_start].0.clone(),
            end_move: moves[end_idx].0.clone(),
            total_eval_drop: total_drop,
            consecutive_moves: streak + 2,
        });
    }

    events
}

// ─── Pipeline Event Callbacks ───

/// Callbacks invoked by the pipeline engine to notify the frontend of progress.
/// All callbacks are optional — the pipeline functions correctly without them.
#[derive(Clone, Default)]
pub struct PipelineCallbacks {
    /// Called for each Stockfish `info depth` line during engine analysis.
    /// Parameters: (depth, eval_cp, nodes)
    pub on_engine_progress: Option<crate::inference::EngineProgressCallback>,
    /// Called when an agent completes its work.
    /// Parameters: (agent_name, status_message)
    #[allow(clippy::type_complexity)]
    pub on_agent_complete: Option<Arc<dyn Fn(&str, &str) + Send + Sync>>,
    /// Called for each streaming token from the pedagogical agent.
    /// Parameters: (token, is_final)
    pub on_streaming_token: Option<crate::inference::StreamingTokenCallback>,
}

/// Defines the type of pipeline to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum PipelineType {
    PostGame,
    LiveCoaching,
    Theory,
    Curriculum,
    Conversational,
}

/// The complete context passed into every pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrchestratorContext {
    pub pipeline_type: PipelineType,
    pub position: FEN,
    pub game_history: Vec<Move>,
    pub engine_output: EngineOutput,
    pub features: FeatureBundle,
    pub rag_results: RetrievalBundle,
    pub user_profile: UserProfile,
    pub persona: Persona,
    pub session: SessionContext,
    /// Optional game PGN for persistence after post-game analysis.
    pub pgn: Option<String>,
    /// Optional game result for persistence.
    pub game_result: Option<String>,
    /// Conversation history for multi-turn chat (Conversational pipeline).
    /// Last entry is the current user message; preceding entries are prior turns.
    pub conversation_history: Vec<crate::agents::ChatHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SessionContext {
    pub session_id: uuid::Uuid,
    pub loaded_model: String,
    pub started_at: String,
}

// ─── Pipeline Definitions (Section 7.3) ───

/// Agents that can participate in a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Type)]
pub enum Agent {
    Tactical,
    Strategic,
    Theory,
    Memory,
    Curriculum,
    Pedagogical,
}

impl Agent {
    /// String name used for error messages and callbacks.
    pub fn as_str(&self) -> &'static str {
        match self {
            Agent::Tactical => "tactical",
            Agent::Strategic => "strategic",
            Agent::Theory => "theory",
            Agent::Memory => "memory",
            Agent::Curriculum => "curriculum",
            Agent::Pedagogical => "pedagogical",
        }
    }
}

/// Returns the ordered agent chain for a given pipeline type.
pub fn agent_chain(pipeline: PipelineType) -> Vec<Agent> {
    match pipeline {
        PipelineType::PostGame => vec![
            Agent::Tactical,
            Agent::Strategic,
            Agent::Theory,
            Agent::Memory,
            Agent::Pedagogical,
        ],
        PipelineType::LiveCoaching => vec![Agent::Tactical, Agent::Pedagogical],
        PipelineType::Theory => vec![Agent::Theory, Agent::Pedagogical],
        PipelineType::Curriculum => vec![Agent::Memory, Agent::Curriculum, Agent::Pedagogical],
        PipelineType::Conversational => vec![Agent::Tactical, Agent::Pedagogical],
    }
}

/// Error handling policy (Section 7.5).
#[derive(Debug, Clone, PartialEq, Type)]
pub enum PipelineError {
    AgentError { agent: String, message: String },
    LlmTimeout { agent: String },
    EngineUnavailable,
    EmptyRagResults,
    LowConfidenceInput,
}

/// Model routing policy (Section 6.5).
/// Returns the model to use for a given pipeline and agent.
pub fn route_model(pipeline: PipelineType, agent: Agent) -> &'static str {
    match (pipeline, agent) {
        (PipelineType::PostGame, _) => "primary", // Qwen3-14B, thinking on
        (PipelineType::LiveCoaching, _) => "fast", // Qwen3-8B, thinking off
        (PipelineType::Theory, _) => "primary",
        (PipelineType::Curriculum, Agent::Curriculum) => "primary",
        (PipelineType::Curriculum, _) => "fast",
        (PipelineType::Conversational, _) => "fast",
    }
}

/// Model swap strategy (Section 6.4).
#[derive(Debug, Clone, PartialEq, Type)]
pub enum SessionPhase {
    GameInProgress,
    PostGameAnalysis,
    AnalysisComplete,
    NewGameStarting,
}

impl SessionPhase {
    /// Returns the model that should be loaded during this phase.
    pub fn recommended_model(&self) -> &str {
        match self {
            SessionPhase::GameInProgress => "fast",
            SessionPhase::PostGameAnalysis => "primary",
            SessionPhase::AnalysisComplete => "primary",
            SessionPhase::NewGameStarting => "fast",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_chain_post_game() {
        let chain = agent_chain(PipelineType::PostGame);
        assert_eq!(
            chain,
            vec![
                Agent::Tactical,
                Agent::Strategic,
                Agent::Theory,
                Agent::Memory,
                Agent::Pedagogical,
            ]
        );
        assert_eq!(chain.len(), 5);
    }

    #[test]
    fn test_agent_chain_live_coaching() {
        let chain = agent_chain(PipelineType::LiveCoaching);
        assert_eq!(chain, vec![Agent::Tactical, Agent::Pedagogical]);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_model_routing_all_pipelines() {
        // PostGame: all agents use primary
        for agent in &[
            Agent::Tactical,
            Agent::Strategic,
            Agent::Theory,
            Agent::Memory,
            Agent::Pedagogical,
        ] {
            assert_eq!(route_model(PipelineType::PostGame, *agent), "primary");
        }
        // LiveCoaching: all agents use fast
        for agent in &[Agent::Tactical, Agent::Pedagogical] {
            assert_eq!(route_model(PipelineType::LiveCoaching, *agent), "fast");
        }
        // Theory: all agents use primary
        assert_eq!(route_model(PipelineType::Theory, Agent::Theory), "primary");
        assert_eq!(
            route_model(PipelineType::Theory, Agent::Pedagogical),
            "primary"
        );
        // Curriculum: curriculum agent uses primary, others use fast
        assert_eq!(
            route_model(PipelineType::Curriculum, Agent::Curriculum),
            "primary"
        );
        assert_eq!(
            route_model(PipelineType::Curriculum, Agent::Memory),
            "fast"
        );
        assert_eq!(
            route_model(PipelineType::Curriculum, Agent::Pedagogical),
            "fast"
        );
        // Conversational: all agents use fast
        assert_eq!(
            route_model(PipelineType::Conversational, Agent::Pedagogical),
            "fast"
        );
    }

    #[test]
    fn test_session_phase_recommendations() {
        assert_eq!(SessionPhase::GameInProgress.recommended_model(), "fast");
        assert_eq!(
            SessionPhase::PostGameAnalysis.recommended_model(),
            "primary"
        );
        assert_eq!(
            SessionPhase::AnalysisComplete.recommended_model(),
            "primary"
        );
        assert_eq!(SessionPhase::NewGameStarting.recommended_model(), "fast");
    }

    #[test]
    fn test_model_routing_post_game() {
        assert_eq!(
            route_model(PipelineType::PostGame, Agent::Tactical),
            "primary"
        );
        assert_eq!(
            route_model(PipelineType::PostGame, Agent::Pedagogical),
            "primary"
        );
    }

    #[test]
    fn test_model_routing_live_coaching() {
        assert_eq!(
            route_model(PipelineType::LiveCoaching, Agent::Tactical),
            "fast"
        );
        assert_eq!(
            route_model(PipelineType::LiveCoaching, Agent::Pedagogical),
            "fast"
        );
    }

    #[test]
    fn test_detect_collapse_no_collapse() {
        let moves = vec![
            ("e2e4".into(), 10, None),
            ("d2d4".into(), 20, None),
            ("g1f3".into(), 30, None),
        ];
        let events = detect_collapse(&moves);
        assert!(events.is_empty());
    }

    #[test]
    fn test_detect_collapse_single_window() {
        let moves = vec![
            ("e2e4".into(), 150, None),
            ("d2d4".into(), 150, None),
            ("g1f3".into(), 50, None),
        ];
        let events = detect_collapse(&moves);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].start_move, "e2e4");
        assert_eq!(events[0].end_move, "g1f3");
        assert!(events[0].total_eval_drop >= 300);
        assert_eq!(events[0].consecutive_moves, 3);
    }

    #[test]
    fn test_detect_collapse_below_threshold() {
        let moves = vec![
            ("e2e4".into(), 50, None),
            ("d2d4".into(), 60, None),
            ("g1f3".into(), 70, None),
        ];
        let events = detect_collapse(&moves);
        assert!(events.is_empty());
    }
}
