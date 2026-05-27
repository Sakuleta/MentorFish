// ─── IPC Layer (Tauri Commands) ───
//
// Section 11.2 of the PRD.
// All frontend-to-backend communication goes through these commands.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Emitter, State};

use crate::engine::EngineManager;
use crate::inference::InferenceClient;
use crate::orchestrator::{CollapseEvent, PipelineCallbacks};

use crate::database::{DatabaseManager, GameStore, ProfileStore};

// ─── Application State ───

/// Shared application state, managed by Tauri.
pub struct AppState {
    pub engine: Box<dyn EngineManager>,
    pub inference: Box<dyn InferenceClient>,
    pub database: Option<Arc<DatabaseManager>>,
}

// ─── Request / Response Types ───

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzePositionRequest {
    pub fen: String,
    pub depth: Option<u32>,
    pub pipeline_type: Option<String>,
    /// Optional natural-language query for RAG retrieval.
    /// If omitted, a query is auto-generated from detected features.
    pub query: Option<String>,
    /// Coaching persona override (defaults to ModernGM if omitted).
    pub persona: Option<String>,
    /// Optional game ID to link analysis moves to a saved game.
    #[serde(default)]
    pub game_id: Option<String>,
    /// Optional analyzed moves to persist when the pipeline completes.
    #[serde(default)]
    pub moves: Option<Vec<crate::Move>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzePositionResponse {
    pub explanation: crate::agents::pedagogical::FinalExplanation,
    pub engine_eval: i32,
    pub best_move: Option<String>,
}

/// Lightweight engine-only analysis request.
/// Does NOT run the LLM pipeline — returns raw engine output instantly.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickAnalyzeRequest {
    pub fen: String,
    pub depth: Option<u32>,
}

/// Lightweight engine-only analysis response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickAnalyzeResponse {
    pub eval_cp: i32,
    pub eval_mate: Option<i32>,
    pub best_move: Option<String>,
    pub depth: u32,
    pub nodes: Option<u64>,
    pub lines: Vec<crate::engine::CandidateLine>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfileResponse {
    pub profile: crate::agents::UserProfile,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub engine_ok: bool,
    pub inference_ok: bool,
    pub database_ok: bool,
}

/// Error report sent from the frontend ErrorBoundary.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorReport {
    pub message: String,
    pub stack: Option<String>,
    pub component: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGameRequest {
    pub pgn: String,
    pub result: Option<String>,
    pub played_at: String,
    pub source: Option<String>,
    pub opening_eco: Option<String>,
    pub time_control: Option<String>,
    /// Optional analyzed moves to persist alongside the game.
    #[serde(default)]
    pub moves: Option<Vec<crate::Move>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameResponse {
    pub game_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPgnRequest {
    /// Raw PGN text containing one or more games.
    pub pgn_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPgnResponse {
    pub games_imported: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPgnRequest {
    /// Optional list of game IDs to export. If omitted, all games are exported.
    pub game_ids: Option<Vec<String>>,
    /// Optional file path to write the PGN to. If omitted, returns the PGN string in the response.
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPgnResponse {
    pub pgn: String,
    pub game_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSummaryResponse {
    pub game_id: String,
    pub opponent: String,
    pub result: String,
    pub played_at: String,
    pub opening: String,
    pub move_count: u32,
}

// ─── Tauri Event Payloads ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingTokenEvent {
    pub token: String,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachingAlertEvent {
    pub alert_type: String,
    pub message: String,
    pub position_fen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineProgressEvent {
    pub depth: u32,
    pub eval_cp: i32,
    pub best_move: Option<String>,
    pub nodes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachingTriggerEvent {
    pub trigger_type: String,
    pub message: String,
    pub severity: String,
    pub position_fen: String,
}

// ─── Chat Types ───

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageRequest {
    pub message: String,
    pub fen: Option<String>,
    pub history: Vec<crate::agents::ChatHistoryEntry>,
    pub persona: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageResponse {
    pub reply: String,
}

// ─── Helpers ───

/// Parse a persona string (from the frontend) into the `Persona` enum.
/// Falls back to `ModernGM` if unrecognized or missing.
fn parse_persona(raw: &Option<String>) -> crate::agents::pedagogical::Persona {
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
fn parse_pipeline_type(raw: &Option<String>) -> crate::orchestrator::PipelineType {
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
fn emit_event<T: Serialize + Clone>(window: &tauri::Window, event: &str, payload: T) {
    if let Err(e) = window.emit(event, payload) {
        log::warn!("Failed to emit '{}' event: {}", event, e);
    }
}

// ─── Conversational Chat Commands ───

/// Send a chat message through the conversational coaching pipeline.
///
/// - If `fen` is provided, runs engine analysis and includes it as context.
/// - Streams tokens via Tauri events (`streaming-token`) for real-time UI updates.
/// - Returns the full reply text in the response.
#[tauri::command]
async fn cmd_chat_message(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: ChatMessageRequest,
) -> Result<ChatMessageResponse, String> {
    let persona = parse_persona(&request.persona);

    log::info!(
        "Chat message (FEN={}): {:.80}...",
        request.fen.as_deref().unwrap_or("none"),
        &request.message
    );

    run_chat_pipeline(&window, state, request, persona, None).await
}

/// Send a chat message through the conversational coaching pipeline,
/// streaming tokens via a Tauri IPC Channel (typed, backpressure-aware).
///
/// This is the preferred variant for production use because the Tauri Channel
/// provides typed deserialization and built-in flow control.
#[tauri::command]
async fn cmd_chat_message_stream(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: ChatMessageRequest,
    on_token: tauri::ipc::Channel<StreamingTokenEvent>,
) -> Result<ChatMessageResponse, String> {
    let persona = parse_persona(&request.persona);

    log::info!(
        "Chat message (channel, FEN={}): {:.80}...",
        request.fen.as_deref().unwrap_or("none"),
        &request.message
    );

    run_chat_pipeline(&window, state, request, persona, Some(on_token)).await
}

/// Shared chat pipeline logic used by both cmd_chat_message and cmd_chat_message_stream.
///
/// When `channel` is `Some`, streaming tokens are sent via the Tauri Channel.
/// When `None`, streaming tokens are emitted via the `streaming-token` window event.
async fn run_chat_pipeline(
    window: &tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: ChatMessageRequest,
    persona: crate::agents::pedagogical::Persona,
    channel: Option<tauri::ipc::Channel<StreamingTokenEvent>>,
) -> Result<ChatMessageResponse, String> {
    let window_for_engine = window.clone();
    let window_for_stream = window.clone();

    // ── 1. Build conversation history including the current message ──
    let mut conversation_history: Vec<crate::agents::ChatHistoryEntry> = request.history.clone();

    // Limit history to 19 previous messages + current = 20 total
    if conversation_history.len() > 19 {
        let skip = conversation_history.len() - 19;
        conversation_history = conversation_history.into_iter().skip(skip).collect();
    }

    // Append the current user message as the last entry
    conversation_history.push(crate::agents::ChatHistoryEntry {
        role: "user".into(),
        content: request.message.clone(),
    });

    // ── 2. Run engine analysis if FEN is provided ──
    let (engine_output, features) = if let Some(ref fen) = request.fen {
        log::info!("Running engine analysis for chat FEN: {}", fen);

        // Emit initial engine progress
        emit_event(
            window,
            "engine-progress",
            EngineProgressEvent {
                depth: 0,
                eval_cp: 0,
                best_move: None,
                nodes: None,
            },
        );

        let engine_progress_cb: Arc<dyn Fn(u32, i32, Option<u64>) + Send + Sync> =
            Arc::new(move |depth, eval_cp, nodes| {
                emit_event(
                    &window_for_engine,
                    "engine-progress",
                    EngineProgressEvent {
                        depth,
                        eval_cp,
                        best_move: None,
                        nodes,
                    },
                );
            });

        let engine_output = state
            .engine
            .analyze(fen, Some(18), Some(engine_progress_cb))
            .await
            .map_err(|e| format!("Engine error: {}", e))?;

        let features = crate::features::extractor::extract_rule_based(fen)
            .map_err(|e| format!("Feature extraction error: {}", e))?;

        (engine_output, features)
    } else {
        // No FEN — use default/empty engine data
        use crate::engine::EngineOutput;
        let engine_output = EngineOutput::default();

        // Use starting position for feature extraction baseline
        let features = crate::features::extractor::extract_rule_based(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap_or_else(|_| crate::features::FeatureBundle {
            position_fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".into(),
            eval_cp: 0,
            eval_swing_cp: 0,
            is_forced_mate: false,
            mate_in: None,
            top_moves: vec![],
            tactics: vec![],
            positional: vec![],
            dynamic: vec![],
            confidence: crate::features::ExtractionConfidence::High,
        });

        (engine_output, features)
    };

    // ── 3. Load or create user profile ──
    let user_profile = load_or_create_profile(state.database.as_deref()).await;

    // ── 4. Build orchestrator context ──
    let ctx = crate::orchestrator::OrchestratorContext {
        pipeline_type: crate::orchestrator::PipelineType::Conversational,
        position: request
            .fen
            .unwrap_or_else(|| "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".into()),
        game_history: vec![],
        engine_output,
        features,
        rag_results: crate::agents::RetrievalBundle {
            chunks: vec![],
            opening_node: None,
            model_games: vec![],
        },
        user_profile: user_profile.clone(),
        persona,
        session: crate::orchestrator::SessionContext {
            session_id: uuid::Uuid::new_v4(),
            loaded_model: "fast".into(),
            started_at: chrono::Utc::now().to_rfc3339(),
        },
        pgn: None,
        game_result: None,
        conversation_history,
    };

    // ── 5. Build streaming callbacks ──
    let on_streaming_token: Option<crate::inference::StreamingTokenCallback> =
        if let Some(chan) = channel {
            Some(Arc::new(move |token: &str, is_final: bool| {
                if let Err(e) = chan.send(StreamingTokenEvent {
                    token: token.to_string(),
                    is_final,
                }) {
                    log::warn!("Failed to send streaming token via channel: {}", e);
                }
            }))
        } else {
            Some(Arc::new(move |token: &str, is_final: bool| {
                emit_event(
                    &window_for_stream,
                    "streaming-token",
                    StreamingTokenEvent {
                        token: token.to_string(),
                        is_final,
                    },
                );
            }))
        };

    let callbacks = PipelineCallbacks {
        on_engine_progress: None,
        on_agent_complete: None,
        on_streaming_token,
    };

    // ── 6. Execute conversational pipeline ──
    let result =
        crate::orchestrator::engine::execute(state.inference.as_ref(), &ctx, callbacks).await;

    Ok(ChatMessageResponse {
        reply: result.explanation.text,
    })
}

// ─── Tauri Command Registrations ───

pub fn register_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        cmd_analyze_position,
        cmd_quick_analyze,
        cmd_stream_analyze,
        cmd_chat_message,
        cmd_chat_message_stream,
        cmd_get_user_profile,
        cmd_generate_curriculum,
        cmd_health_check,
        cmd_run_ingestion,
        cmd_get_knowledge_summary,
        cmd_copy_to_knowledge,
        cmd_make_move,
        cmd_ai_move,
        cmd_get_legal_moves,
        cmd_detect_collapse,
        cmd_save_game,
        cmd_get_game_moves,
        cmd_get_opening,
        cmd_get_recent_games,
        cmd_import_pgn,
        cmd_export_pgn,
        cmd_get_config,
        cmd_save_config,
        cmd_get_book_chunks,
        cmd_report_error,
    ])
}

// ─── Command Implementations ───

#[tauri::command]
async fn cmd_analyze_position(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: AnalyzePositionRequest,
) -> Result<AnalyzePositionResponse, String> {
    let fen_for_events = request.fen.clone();
    let pipeline_type = parse_pipeline_type(&request.pipeline_type);

    log::info!(
        "Analyzing position with pipeline {:?}: {}",
        pipeline_type,
        &request.fen
    );

    // ── Emit initial progress ──
    emit_event(
        &window,
        "engine-progress",
        EngineProgressEvent {
            depth: 0,
            eval_cp: 0,
            best_move: None,
            nodes: None,
        },
    );

    // ── Build engine progress callback ──
    let window_for_engine = window.clone();
    let engine_progress_cb: std::sync::Arc<dyn Fn(u32, i32, Option<u64>) + Send + Sync> =
        Arc::new(move |depth, eval_cp, nodes| {
            emit_event(
                &window_for_engine,
                "engine-progress",
                EngineProgressEvent {
                    depth,
                    eval_cp,
                    best_move: None,
                    nodes,
                },
            );
        });

    // Run engine analysis (clone the callback so we can reuse it in the pipeline)
    let engine_output = state
        .engine
        .analyze(
            &request.fen,
            request.depth,
            Some(engine_progress_cb.clone()),
        )
        .await
        .map_err(|e| format!("Engine error: {}", e))?;

    // Extract features (rule-based, fast)
    let features = crate::features::extractor::extract_rule_based(&request.fen)
        .map_err(|e| format!("Feature extraction error: {}", e))?;

    // ── RAG Retrieval ──
    // Generate a default query from detected features if none provided
    let query = request
        .query
        .unwrap_or_else(|| build_default_query(&features));

    // Compute embedding for semantic search
    let query_embedding = state.inference.embed(&query).await.ok();

    let retrieval_result =
        crate::knowledge::retrieve(&request.fen, &query, 5, query_embedding.as_deref()).await;
    let opening_node = crate::knowledge::retrieve_opening(&request.fen)
        .await
        .unwrap_or(None);

    let rag_chunks = retrieval_result.map(|r| r.chunks).unwrap_or_default();

    log::info!(
        "RAG retrieved {} chunks (query: '{}'), opening: {}",
        rag_chunks.len(),
        &query,
        opening_node
            .as_ref()
            .and_then(|n| n.opening_name.as_deref())
            .unwrap_or("none")
    );

    // ── Load or create user profile ──
    let user_profile = load_or_create_profile(state.database.as_deref()).await;

    // Build orchestrator context
    let ctx = crate::orchestrator::OrchestratorContext {
        pipeline_type,
        position: request.fen.clone(),
        game_history: vec![],
        engine_output,
        features,
        rag_results: crate::agents::RetrievalBundle {
            chunks: rag_chunks,
            opening_node,
            model_games: vec![],
        },
        user_profile: user_profile.clone(),
        persona: parse_persona(&request.persona),
        session: crate::orchestrator::SessionContext {
            session_id: uuid::Uuid::new_v4(),
            loaded_model: "primary".into(),
            started_at: chrono::Utc::now().to_rfc3339(),
        },
        pgn: None,
        game_result: None,
        conversation_history: vec![],
    };

    // Capture values before context is consumed
    let eval_cp = ctx.features.eval_cp;
    let best_move = ctx.engine_output.best_move.clone();

    // ── Build pipeline callbacks for live frontend updates ──
    let window_for_pipeline = window.clone();
    let window_for_stream = window.clone();
    let fen_clone = fen_for_events.clone();

    // Wire the existing engine-progress callback into the pipeline so that any
    // agent that needs additional engine analysis during the pipeline can also
    // emit progress to the frontend.
    let on_engine_progress: Option<crate::inference::EngineProgressCallback> =
        Some(engine_progress_cb.clone());

    let callbacks = PipelineCallbacks {
        on_engine_progress,
        on_agent_complete: Some(Arc::new(move |agent: &str, message: &str| {
            emit_event(
                &window_for_pipeline,
                "coaching-alert",
                CoachingAlertEvent {
                    alert_type: agent.to_string(),
                    message: message.to_string(),
                    position_fen: fen_clone.clone(),
                },
            );
        })),
        on_streaming_token: Some(Arc::new(move |token: &str, is_final: bool| {
            emit_event(
                &window_for_stream,
                "streaming-token",
                StreamingTokenEvent {
                    token: token.to_string(),
                    is_final,
                },
            );
        })),
    };

    // Execute the pipeline
    let result =
        crate::orchestrator::engine::execute(state.inference.as_ref(), &ctx, callbacks).await;

    // Apply memory delta to user profile
    if let Some(ref delta) = result.profile_delta {
        let updated_profile = crate::memory::apply_delta(&user_profile, delta);
        if let Some(db) = state.database.as_deref() {
            if let Err(e) = db.save_profile(&updated_profile).await {
                log::warn!("Failed to save updated user profile: {}", e);
            }
        }
    }

    // Persist analyzed moves for PostGame pipelines
    if pipeline_type == crate::orchestrator::PipelineType::PostGame {
        if let Some(db) = state.database.as_deref() {
            if let Some(ref game_id_str) = request.game_id {
                if let Ok(game_id) = game_id_str.parse::<uuid::Uuid>() {
                    if let Some(ref moves) = request.moves {
                        if !moves.is_empty() {
                            if let Err(e) = db.save_moves(game_id, moves).await {
                                log::warn!("Failed to persist analysis moves: {}", e);
                            } else {
                                log::info!(
                                    "Persisted {} analyzed moves for game {}",
                                    moves.len(),
                                    game_id
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(AnalyzePositionResponse {
        explanation: result.explanation,
        engine_eval: eval_cp,
        best_move,
    })
}

/// Lightweight engine-only analysis.
///
/// Runs Stockfish analysis and returns the raw engine output (including MultiPV
/// candidate lines).  Does NOT invoke the LLM pipeline, so it is fast and safe
/// to call even when Ollama is unavailable.
#[tauri::command]
async fn cmd_quick_analyze(
    state: State<'_, Arc<AppState>>,
    request: QuickAnalyzeRequest,
) -> Result<QuickAnalyzeResponse, String> {
    let depth = request.depth.unwrap_or(18);
    let engine_output = state
        .engine
        .analyze(&request.fen, Some(depth), None)
        .await
        .map_err(|e| format!("Engine error: {}", e))?;

    Ok(QuickAnalyzeResponse {
        eval_cp: engine_output.eval_cp,
        eval_mate: engine_output.eval_mate,
        best_move: engine_output.best_move,
        depth: engine_output.depth,
        nodes: engine_output.nodes,
        lines: engine_output.multipv,
    })
}

/// Alternative streaming endpoint that uses Tauri's `ipc::Channel` for typed,
/// backpressure-aware token delivery to the frontend.
///
/// This is the recommended approach for new frontend integrations. The existing
/// `cmd_analyze_position` command (which uses `window.emit`) continues to work
/// for backwards compatibility.
#[tauri::command]
async fn cmd_stream_analyze(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: AnalyzePositionRequest,
    on_token: tauri::ipc::Channel<StreamingTokenEvent>,
) -> Result<AnalyzePositionResponse, String> {
    let fen_for_events = request.fen.clone();
    let pipeline_type = parse_pipeline_type(&request.pipeline_type);

    log::info!(
        "Stream-analyzing position with pipeline {:?}: {}",
        pipeline_type,
        &request.fen
    );

    // ── Emit initial progress (via window emit since channels only cover tokens) ──
    emit_event(
        &window,
        "engine-progress",
        EngineProgressEvent {
            depth: 0,
            eval_cp: 0,
            best_move: None,
            nodes: None,
        },
    );

    // ── Build engine progress callback ──
    let window_for_engine = window.clone();
    let engine_progress_cb: std::sync::Arc<dyn Fn(u32, i32, Option<u64>) + Send + Sync> =
        Arc::new(move |depth, eval_cp, nodes| {
            emit_event(
                &window_for_engine,
                "engine-progress",
                EngineProgressEvent {
                    depth,
                    eval_cp,
                    best_move: None,
                    nodes,
                },
            );
        });

    // Run engine analysis
    let engine_output = state
        .engine
        .analyze(
            &request.fen,
            request.depth,
            Some(engine_progress_cb.clone()),
        )
        .await
        .map_err(|e| format!("Engine error: {}", e))?;

    // Extract features (rule-based, fast)
    let features = crate::features::extractor::extract_rule_based(&request.fen)
        .map_err(|e| format!("Feature extraction error: {}", e))?;

    // ── RAG Retrieval ──
    let query = request
        .query
        .unwrap_or_else(|| build_default_query(&features));

    // Compute embedding for semantic search
    let query_embedding = state.inference.embed(&query).await.ok();

    let retrieval_result =
        crate::knowledge::retrieve(&request.fen, &query, 5, query_embedding.as_deref()).await;
    let opening_node = crate::knowledge::retrieve_opening(&request.fen)
        .await
        .unwrap_or(None);

    let rag_chunks = retrieval_result.map(|r| r.chunks).unwrap_or_default();

    log::info!(
        "RAG retrieved {} chunks (query: '{}'), opening: {}",
        rag_chunks.len(),
        &query,
        opening_node
            .as_ref()
            .and_then(|n| n.opening_name.as_deref())
            .unwrap_or("none")
    );

    // ── Load or create user profile ──
    let user_profile = load_or_create_profile(state.database.as_deref()).await;

    // Build orchestrator context
    let ctx = crate::orchestrator::OrchestratorContext {
        pipeline_type,
        position: request.fen.clone(),
        game_history: vec![],
        engine_output,
        features,
        rag_results: crate::agents::RetrievalBundle {
            chunks: rag_chunks,
            opening_node,
            model_games: vec![],
        },
        user_profile: user_profile.clone(),
        persona: parse_persona(&request.persona),
        session: crate::orchestrator::SessionContext {
            session_id: uuid::Uuid::new_v4(),
            loaded_model: "primary".into(),
            started_at: chrono::Utc::now().to_rfc3339(),
        },
        pgn: None,
        game_result: None,
        conversation_history: vec![],
    };

    // Capture values before context is consumed
    let eval_cp = ctx.features.eval_cp;
    let best_move = ctx.engine_output.best_move.clone();

    // ── Build pipeline callbacks ──
    // Uses Tauri Channel for streaming tokens (typed, backpressure-aware),
    // and window.emit for agent progress / engine progress events.
    let window_for_alerts = window.clone();
    let fen_clone = fen_for_events.clone();
    let on_engine_progress: Option<crate::inference::EngineProgressCallback> =
        Some(engine_progress_cb);

    let callbacks = PipelineCallbacks {
        on_engine_progress,
        on_agent_complete: Some(Arc::new(move |agent: &str, message: &str| {
            emit_event(
                &window_for_alerts,
                "coaching-alert",
                CoachingAlertEvent {
                    alert_type: agent.to_string(),
                    message: message.to_string(),
                    position_fen: fen_clone.clone(),
                },
            );
        })),
        on_streaming_token: Some(Arc::new(move |token: &str, is_final: bool| {
            // Use Tauri's typed Channel for streaming tokens
            if let Err(e) = on_token.send(StreamingTokenEvent {
                token: token.to_string(),
                is_final,
            }) {
                log::warn!(
                    "Failed to send streaming token via channel (frontend may have disconnected): {}",
                    e
                );
            }
        })),
    };

    // Execute the pipeline
    let result =
        crate::orchestrator::engine::execute(state.inference.as_ref(), &ctx, callbacks).await;

    // ── Apply memory delta to user profile ──
    if let Some(ref delta) = result.profile_delta {
        let updated_profile = crate::memory::apply_delta(&user_profile, delta);
        if let Some(db) = state.database.as_deref() {
            if let Err(e) = db.save_profile(&updated_profile).await {
                log::warn!("Failed to save updated user profile: {}", e);
            }
        }
    }

    Ok(AnalyzePositionResponse {
        explanation: result.explanation,
        engine_eval: eval_cp,
        best_move,
    })
}

#[tauri::command]
async fn cmd_get_user_profile(
    state: State<'_, Arc<AppState>>,
) -> Result<UserProfileResponse, String> {
    let profile = load_or_create_profile(state.database.as_deref()).await;
    Ok(UserProfileResponse { profile })
}

#[tauri::command]
async fn cmd_generate_curriculum(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::agents::curriculum::StudyPlan, String> {
    let db = state.database.as_deref();
    let profile = load_or_create_profile(db).await;

    let plan = crate::agents::executor::run_curriculum(state.inference.as_ref(), &profile)
        .await
        .map_err(|e| e.to_string())?;

    Ok(plan)
}

#[tauri::command]
async fn cmd_get_recent_games(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<GameSummaryResponse>, String> {
    let db = match state.database.as_deref() {
        Some(db) => db,
        None => return Ok(vec![]),
    };

    let user_id = db
        .default_user_id()
        .await
        .map_err(|e| format!("Failed to get user ID: {}", e))?;

    let games = db
        .get_user_games(user_id, 10)
        .await
        .map_err(|e| format!("Failed to load recent games: {}", e))?;

    let summaries: Vec<GameSummaryResponse> = games
        .into_iter()
        .map(|g| {
            // Estimate move count from PGN (count periods which separate move numbers)
            let move_count = g.pgn.split('.').count().saturating_sub(1) as u32;

            // Extract opponent from PGN header if available
            let opponent = if let Some(black_tag) = g.pgn.lines().find(|l| l.starts_with("[Black "))
            {
                black_tag
                    .trim_start_matches("[Black ")
                    .trim_end_matches(']')
                    .trim_matches('"')
                    .to_string()
            } else if let Some(white_tag) = g.pgn.lines().find(|l| l.starts_with("[White ")) {
                white_tag
                    .trim_start_matches("[White ")
                    .trim_end_matches(']')
                    .trim_matches('"')
                    .to_string()
            } else {
                "Unknown".to_string()
            };

            GameSummaryResponse {
                game_id: g.game_id.to_string(),
                opponent,
                result: g.result.unwrap_or_else(|| "*".to_string()),
                played_at: g.played_at,
                opening: g.opening_eco.unwrap_or_else(|| "—".to_string()),
                move_count,
            }
        })
        .collect();

    Ok(summaries)
}

#[tauri::command]
async fn cmd_health_check(state: State<'_, Arc<AppState>>) -> Result<HealthCheckResponse, String> {
    let engine_ok = state.engine.health_check().await.unwrap_or(false);

    // Check inference by verifying Ollama is reachable
    let inference_ok = state.inference.health_check().await.unwrap_or(false);

    let database_ok = state.database.is_some();

    Ok(HealthCheckResponse {
        engine_ok,
        inference_ok,
        database_ok,
    })
}

/// Accept an error report from the frontend ErrorBoundary and write it to the log.
#[tauri::command]
async fn cmd_report_error(report: ErrorReport) -> Result<(), String> {
    log::error!(
        "Frontend error [{}]: {} (stack: {})",
        report.component.as_deref().unwrap_or("unknown"),
        report.message,
        report.stack.as_deref().unwrap_or("none")
    );
    Ok(())
}

#[tauri::command]
async fn cmd_save_game(
    state: State<'_, Arc<AppState>>,
    request: SaveGameRequest,
) -> Result<SaveGameResponse, String> {
    let db = state
        .database
        .as_deref()
        .ok_or_else(|| "Database not available".to_string())?;

    let user_id = db
        .default_user_id()
        .await
        .map_err(|e| format!("Failed to get user ID: {}", e))?;

    let game_id = uuid::Uuid::new_v4();
    let record = crate::memory::GameRecord {
        game_id,
        user_id,
        pgn: request.pgn,
        result: request.result,
        played_at: request.played_at,
        source: request.source.unwrap_or_else(|| "manual".into()),
        opening_eco: request.opening_eco,
        time_control: request.time_control,
    };

    db.save_game(record)
        .await
        .map_err(|e| format!("Failed to save game: {}", e))?;

    // Persist analyzed moves if provided
    if let Some(moves) = &request.moves {
        if !moves.is_empty() {
            db.save_moves(game_id, moves)
                .await
                .map_err(|e| format!("Failed to save moves: {}", e))?;
            log::info!("Saved {} moves for game {}", moves.len(), game_id);
        }
    }

    Ok(SaveGameResponse {
        game_id: game_id.to_string(),
    })
}

#[tauri::command]
async fn cmd_get_game_moves(
    state: State<'_, Arc<AppState>>,
    game_id: String,
) -> Result<Vec<crate::Move>, String> {
    let db = state
        .database
        .as_deref()
        .ok_or_else(|| "Database not available".to_string())?;
    let id: uuid::Uuid = game_id
        .parse()
        .map_err(|e| format!("Invalid game ID: {}", e))?;
    db.get_game_moves(id)
        .await
        .map_err(|e| format!("Failed to get moves: {}", e))
}

// ─── PGN Import / Export (Section 8.2) ───

#[tauri::command]
async fn cmd_import_pgn(
    state: State<'_, Arc<AppState>>,
    request: ImportPgnRequest,
) -> Result<ImportPgnResponse, String> {
    let db = state
        .database
        .as_deref()
        .ok_or_else(|| "Database not available".to_string())?;

    let user_id = db
        .default_user_id()
        .await
        .map_err(|e| format!("Failed to get user ID: {}", e))?;

    let parsed = crate::memory::parse_pgn_games(&request.pgn_text);

    let mut games_imported: u32 = 0;
    let mut errors: Vec<String> = Vec::new();

    for game in &parsed {
        // Rebuild canonical PGN from headers + movetext
        let mut pgn_builder = String::new();
        for (key, value) in &game.headers {
            pgn_builder.push_str(&format!("[{} \"{}\"]\n", key, value));
        }
        pgn_builder.push('\n');
        pgn_builder.push_str(&game.movetext);
        pgn_builder.push('\n');

        // Extract standard header fields
        let result = game
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Result"))
            .map(|(_, v)| v.clone());

        let eco = game
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("ECO"))
            .map(|(_, v)| v.clone());

        let date = game
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Date"))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| chrono::Utc::now().format("%Y.%m.%d").to_string());

        let game_id = uuid::Uuid::new_v4();
        let record = crate::memory::GameRecord {
            game_id,
            user_id,
            pgn: pgn_builder,
            result,
            played_at: date,
            source: "imported".into(),
            opening_eco: eco,
            time_control: None,
        };

        match db.save_game(record).await {
            Ok(_) => games_imported += 1,
            Err(e) => errors.push(format!("Failed to save game: {}", e)),
        }
    }

    Ok(ImportPgnResponse {
        games_imported,
        errors,
    })
}

#[tauri::command]
async fn cmd_export_pgn(
    state: State<'_, Arc<AppState>>,
    request: ExportPgnRequest,
) -> Result<ExportPgnResponse, String> {
    let db = state
        .database
        .as_deref()
        .ok_or_else(|| "Database not available".to_string())?;

    let user_id = db
        .default_user_id()
        .await
        .map_err(|e| format!("Failed to get user ID: {}", e))?;

    let records = if let Some(ref ids) = request.game_ids {
        let mut games = Vec::with_capacity(ids.len());
        for id_str in ids {
            let game_id = uuid::Uuid::parse_str(id_str)
                .map_err(|e| format!("Invalid game ID '{}': {}", id_str, e))?;
            let record = db
                .get_game(game_id)
                .await
                .map_err(|e| format!("Failed to fetch game '{}': {}", id_str, e))?;
            if let Some(rec) = record {
                games.push(rec);
            }
        }
        games
    } else {
        db.get_all_user_games(user_id)
            .await
            .map_err(|e| format!("Failed to fetch games: {}", e))?
    };

    let mut pgn_output = String::new();
    for record in &records {
        pgn_output.push_str(&record.pgn);
        pgn_output.push_str("\n\n");
    }

    let game_count = records.len() as u32;

    // Write to file if a path was provided
    if let Some(ref path) = request.output_path {
        std::fs::write(path, &pgn_output)
            .map_err(|e| format!("Failed to write PGN to '{}': {}", path, e))?;
    }

    Ok(ExportPgnResponse {
        pgn: pgn_output,
        game_count,
    })
}

// ─── Play Commands ───

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MakeMoveRequest {
    pub fen: String,
    pub uci: String,
    pub vs_ai: bool,
    /// Strength mode: "full" (default), "stockfish_elo", "boltzmann", or "training"
    #[serde(default)]
    pub strength_mode: Option<String>,
    /// Target ELO for strength-limited modes (1320..3190 for stockfish_elo)
    #[serde(default)]
    pub target_elo: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MakeMoveResponse {
    pub fen: String,
    pub is_check: bool,
    pub is_checkmate: bool,
    pub is_stalemate: bool,
    pub ai_move: Option<String>,
    pub ai_fen: Option<String>,
}

/// Parse strength fields from the frontend request into a PlayStrength variant.
/// Backward-compatible: returns FullStrength if no mode is specified.
fn parse_strength(
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

#[tauri::command]
async fn cmd_make_move(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: MakeMoveRequest,
) -> Result<MakeMoveResponse, String> {
    // --- Live coaching: capture pre-move evaluation (depth 8, cheap) ---
    // Deep pre-move analysis would add 5-15s latency before we even process the
    // user's move, making the game feel unresponsive. Use a very fast depth 6
    // eval just for coaching context. The coaching check runs its own analysis.
    let prev_output = state.engine.analyze(&request.fen, Some(6), None).await.ok();
    let prev_eval_cp = prev_output.as_ref().map(|o| o.eval_cp);

    // Parse move number from FEN (last field is the fullmove counter)
    let move_number = request
        .fen
        .split_whitespace()
        .last()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    if request.vs_ai {
        let strength = parse_strength(&request.strength_mode, request.target_elo);

        // Load user profile for training mode (weakness-pattern matching)
        let user_profile = if matches!(strength, crate::engine::play::PlayStrength::Training) {
            Some(load_or_create_profile(state.database.as_deref()).await)
        } else {
            None
        };

        let result = crate::engine::play::play_vs_stockfish(
            state.engine.as_ref(),
            &request.fen,
            Some(&request.uci),
            &strength,
            user_profile.as_ref(),
        )
        .await
        .map_err(|e| format!("Move error: {}", e))?;

        // --- Live coaching: quick analysis of the resulting position ---
        run_coaching_check(
            &window,
            state.engine.as_ref(),
            prev_eval_cp,
            &result.fen_after,
            move_number,
            state.database.as_deref(),
        )
        .await;

        Ok(MakeMoveResponse {
            fen: result.fen_after,
            is_check: result.is_check,
            is_checkmate: result.is_checkmate,
            is_stalemate: result.is_stalemate,
            ai_move: result.ai_move,
            ai_fen: result.ai_fen,
        })
    } else {
        let result = crate::engine::play::apply_move(&request.fen, &request.uci)
            .map_err(|e| format!("Move error: {}", e))?;

        // --- Live coaching: quick analysis of the resulting position ---
        run_coaching_check(
            &window,
            state.engine.as_ref(),
            prev_eval_cp,
            &result.fen_after,
            move_number,
            state.database.as_deref(),
        )
        .await;

        Ok(MakeMoveResponse {
            fen: result.fen_after,
            is_check: result.is_check,
            is_checkmate: result.is_checkmate,
            is_stalemate: result.is_stalemate,
            ai_move: None,
            ai_fen: None,
        })
    }
}

/// Ask Stockfish to make a move for the given position (AI plays first).
/// Strength mode: "full" (default), "stockfish_elo", "boltzmann", or "training"
#[tauri::command]
async fn cmd_ai_move(
    state: State<'_, Arc<AppState>>,
    fen: String,
    strength_mode: Option<String>,
    target_elo: Option<u32>,
) -> Result<MakeMoveResponse, String> {
    let strength = parse_strength(&strength_mode, target_elo);

    let result =
        crate::engine::play::play_vs_stockfish(state.engine.as_ref(), &fen, None, &strength, None)
            .await
            .map_err(|e| format!("AI move error: {}", e))?;

    Ok(MakeMoveResponse {
        fen: result.fen_after,
        is_check: result.is_check,
        is_checkmate: result.is_checkmate,
        is_stalemate: result.is_stalemate,
        ai_move: result.ai_move,
        ai_fen: result.ai_fen,
    })
}

/// Run live coaching trigger checks after the user's move.
/// Emits any triggered events to the frontend via `coaching-trigger`.
async fn run_coaching_check(
    window: &tauri::Window,
    engine: &dyn EngineManager,
    prev_eval_cp: Option<i32>,
    fen_after: &str,
    move_number: u32,
    db: Option<&DatabaseManager>,
) {
    // Quick engine analysis of the position after the user's move
    // Depth 8 is sufficient for blunder/mistake detection; deeper analysis
    // would delay the game flow noticeably.
    let post_output = match engine.analyze(&fen_after.to_string(), Some(8), None).await {
        Ok(o) => o,
        Err(e) => {
            log::warn!("Coaching engine analysis failed: {}", e);
            return;
        }
    };

    // Extract rule-based features and fill in eval values from the quick analysis
    let mut features = match crate::features::extractor::extract_rule_based(fen_after) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Coaching feature extraction failed: {}", e);
            return;
        }
    };
    features.eval_cp = post_output.eval_cp;
    features.eval_swing_cp = prev_eval_cp
        .map(|prev| post_output.eval_cp - prev)
        .unwrap_or(0);

    // Load user profile
    let user_profile = load_or_create_profile(db).await;

    // Check coaching triggers
    let triggers = crate::orchestrator::check_coaching_triggers(
        fen_after,
        prev_eval_cp,
        &post_output,
        &features,
        &user_profile,
        move_number,
        None, // remaining_time_ms — clock not yet implemented
        None, // total_time_ms — clock not yet implemented
    );

    // Emit each triggered event to the frontend
    for trigger in &triggers {
        emit_event(
            window,
            "coaching-trigger",
            CoachingTriggerEvent {
                trigger_type: trigger.trigger_type.clone(),
                message: trigger.message.clone(),
                severity: trigger.severity.clone(),
                position_fen: trigger.position_fen.clone(),
            },
        );
    }

    if !triggers.is_empty() {
        log::info!(
            "Emitted {} coaching trigger(s) for move {}",
            triggers.len(),
            move_number
        );
    }
}

#[tauri::command]
async fn cmd_get_legal_moves(fen: String) -> Result<Vec<String>, String> {
    crate::engine::play::get_legal_moves(&fen).map_err(|e| format!("{}", e))
}

// ─── Psychological Collapse Detection ───

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollapseDetectRequest {
    /// Move history: each entry is (uci, eval_swing_cp, move_time_ms)
    pub moves: Vec<(String, i32, Option<u64>)>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollapseDetectResponse {
    pub collapses: Vec<CollapseEvent>,
}

#[tauri::command]
async fn cmd_detect_collapse(
    request: CollapseDetectRequest,
) -> Result<CollapseDetectResponse, String> {
    let collapses = crate::orchestrator::detect_collapse(&request.moves);
    Ok(CollapseDetectResponse { collapses })
}

// ─── Opening Explorer ───

#[derive(Debug, Serialize, Deserialize)]
pub struct OpeningNodeResponse {
    pub node: Option<crate::agents::OpeningNode>,
}

#[tauri::command]
async fn cmd_get_opening(fen: String) -> Result<OpeningNodeResponse, String> {
    let node = crate::knowledge::retrieve_opening(&fen)
        .await
        .map_err(|e| format!("{}", e))?;
    Ok(OpeningNodeResponse { node })
}

// ─── Knowledge Ingestion ───

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestionReportResponse {
    pub books_processed: u64,
    pub chunks_created: u64,
    pub chunks_embedded: u64,
    pub message: String,
}

// ─── Knowledge Summary ───

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeSummary {
    pub total_books: u64,
    pub total_chunks: u64,
    pub total_embedded: u64,
    pub books: Vec<BookSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookSummary {
    pub title: String,
    pub chunk_count: u64,
    pub chunk_type: String,
    pub has_embeddings: bool,
}

// ─── File Import ───

#[derive(Debug, Deserialize)]
pub struct CopyToKnowledgeRequest {
    pub file_name: String,
    pub file_content: Vec<u8>,
    pub file_type: String,
}

#[tauri::command]
async fn cmd_run_ingestion(
    state: State<'_, Arc<AppState>>,
) -> Result<IngestionReportResponse, String> {
    let knowledge_dir = crate::knowledge::knowledge_dir().to_path_buf();
    crate::knowledge::ingestion::run_ingestion(state.inference.as_ref(), &knowledge_dir)
        .await
        .map(|report| IngestionReportResponse {
            books_processed: report.books_processed,
            chunks_created: report.chunks_created,
            chunks_embedded: report.chunks_embedded,
            message: report.message,
        })
        .map_err(|e| format!("Ingestion error: {}", e))
}

#[tauri::command]
async fn cmd_get_knowledge_summary() -> Result<KnowledgeSummary, String> {
    let knowledge_dir = crate::knowledge::knowledge_dir();

    // Try chunks_indexed.json first (has embeddings), then chunks_all.json
    let chunks_path = if knowledge_dir.join("chunks_indexed.json").exists() {
        knowledge_dir.join("chunks_indexed.json")
    } else if knowledge_dir.join("chunks_all.json").exists() {
        knowledge_dir.join("chunks_all.json")
    } else {
        return Ok(KnowledgeSummary {
            total_books: 0,
            total_chunks: 0,
            total_embedded: 0,
            books: vec![],
        });
    };

    let json = std::fs::read_to_string(&chunks_path)
        .map_err(|e| format!("Failed to read chunks: {}", e))?;
    let root: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("Invalid JSON: {}", e))?;

    let chunks = root["chunks"]
        .as_array()
        .ok_or_else(|| "Missing 'chunks' array in chunks file".to_string())?;

    // Group chunks by source
    let mut source_map: std::collections::HashMap<String, (u64, String, bool)> =
        std::collections::HashMap::new();
    let mut total_embedded = 0u64;

    for chunk in chunks {
        let source = chunk["source"].as_str().unwrap_or("Unknown").to_string();
        let chunk_type = chunk["chunk_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let has_embedding = chunk["embedding"].is_array()
            && !chunk["embedding"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(true);

        if has_embedding {
            total_embedded += 1;
        }

        let entry = source_map
            .entry(source.clone())
            .or_insert((0, chunk_type, false));
        entry.0 += 1;
        entry.2 = entry.2 || has_embedding;
    }

    let books: Vec<BookSummary> = source_map
        .into_iter()
        .map(|(title, (count, chunk_type, has_embeddings))| BookSummary {
            title,
            chunk_count: count,
            chunk_type,
            has_embeddings,
        })
        .collect();

    Ok(KnowledgeSummary {
        total_books: books.len() as u64,
        total_chunks: chunks.len() as u64,
        total_embedded,
        books,
    })
}

#[tauri::command]
async fn cmd_copy_to_knowledge(request: CopyToKnowledgeRequest) -> Result<String, String> {
    let knowledge_dir = crate::knowledge::knowledge_dir();
    let dest_dir = match request.file_type.as_str() {
        "pdf" => knowledge_dir.join("books"),
        "pgn" => knowledge_dir.join("pgn"),
        other => {
            return Err(format!(
                "Invalid file type '{}'. Use 'pdf' or 'pgn'.",
                other
            ))
        }
    };

    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Sanitize filename: only keep the base name, strip any path components
    let filename = std::path::Path::new(&request.file_name)
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?;
    let dest = dest_dir.join(filename);

    std::fs::write(&dest, &request.file_content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!(
        "Imported {} file: {} → {}",
        request.file_type,
        request.file_name,
        dest.display()
    );

    Ok(dest.to_string_lossy().to_string())
}

// ─── Helpers ───

/// Build a default RAG query string from detected positional and tactical features.
fn build_default_query(features: &crate::features::FeatureBundle) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Extract feature descriptions
    for t in &features.tactics {
        match t {
            crate::features::TacticalFeature::Fork { .. } => parts.push("fork".into()),
            crate::features::TacticalFeature::Pin { .. } => parts.push("pin".into()),
            crate::features::TacticalFeature::Skewer { .. } => parts.push("skewer".into()),
            crate::features::TacticalFeature::DiscoveredAttack { .. } => {
                parts.push("discovered attack".into())
            }
            crate::features::TacticalFeature::HangingPiece { .. } => {
                parts.push("hanging piece".into())
            }
        }
    }

    for p in &features.positional {
        match p {
            crate::features::PositionalFeature::IsolatedPawn { .. } => {
                parts.push("isolated pawn".into())
            }
            crate::features::PositionalFeature::DoubledPawn { .. } => {
                parts.push("doubled pawns".into())
            }
            crate::features::PositionalFeature::PassedPawn { .. } => {
                parts.push("passed pawn".into())
            }
            crate::features::PositionalFeature::OpenFile { .. } => parts.push("open file".into()),
            crate::features::PositionalFeature::HalfOpenFile { .. } => {
                parts.push("half open file".into())
            }
            crate::features::PositionalFeature::BishopPair { .. } => {
                parts.push("bishop pair".into())
            }
            crate::features::PositionalFeature::Outpost { .. } => {
                parts.push("outpost square".into())
            }
            crate::features::PositionalFeature::BackwardPawn { .. } => {
                parts.push("backward pawn".into())
            }
            crate::features::PositionalFeature::KingSafety { .. } => {
                parts.push("king safety".into())
            }
            crate::features::PositionalFeature::PawnIsland { .. } => {
                parts.push("pawn structure".into())
            }
        }
    }

    for d in &features.dynamic {
        match d {
            crate::features::DynamicFeature::PieceMobility { .. } => {
                parts.push("piece mobility".into())
            }
            crate::features::DynamicFeature::SpaceAdvantage { .. } => {
                parts.push("space advantage".into())
            }
            crate::features::DynamicFeature::Development { .. } => parts.push("development".into()),
            crate::features::DynamicFeature::Initiative { .. } => parts.push("initiative".into()),
        }
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
fn detect_game_phase(features: &crate::features::FeatureBundle) -> &'static str {
    let fen = &features.position_fen;
    let board_part = fen.split_whitespace().next().unwrap_or("");
    let piece_count: usize = board_part.chars().filter(|c| c.is_uppercase()).count();
    let piece_count_b: usize = board_part
        .chars()
        .filter(|c| c.is_lowercase() && *c != 'p')
        .count();
    let total_pieces = piece_count + piece_count_b;

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
async fn load_or_create_profile(db: Option<&DatabaseManager>) -> crate::agents::UserProfile {
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

// ─── Config Commands ───

#[tauri::command]
async fn cmd_get_config() -> Result<crate::config::AppConfig, String> {
    crate::config::AppConfig::load().map_err(|e| format!("Failed to load config: {}", e))
}

#[tauri::command]
async fn cmd_save_config(config: crate::config::AppConfig) -> Result<(), String> {
    config
        .save()
        .map_err(|e| format!("Failed to save config: {}", e))
}

// ─── Book Reader Commands (Section 3.7) ───

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBookChunksRequest {
    /// Book title / source name to filter chunks by.
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBookChunksResponse {
    pub chunks: Vec<crate::agents::KnowledgeChunk>,
}

#[tauri::command]
async fn cmd_get_book_chunks(
    request: GetBookChunksRequest,
) -> Result<GetBookChunksResponse, String> {
    let all = crate::knowledge::retrieve_by_source(&request.source, 100_000)
        .await
        .map_err(|e| format!("Failed to retrieve book chunks: {}", e))?;

    Ok(GetBookChunksResponse { chunks: all.chunks })
}
