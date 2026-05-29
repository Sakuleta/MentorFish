// ─── Analysis Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;

use crate::database::{GameStore, ProfileStore};
use crate::orchestrator::PipelineCallbacks;

#[tauri::command]
pub async fn cmd_analyze_position(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: AnalyzePositionRequest,
) -> Result<AnalyzePositionResponse, String> {
    // Rate limit: only 2 concurrent analyses allowed
    let _permit = state
        .analysis_semaphore
        .acquire()
        .await
        .map_err(|_| "Analysis rate limit exceeded".to_string())?;

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
pub async fn cmd_quick_analyze(
    state: State<'_, Arc<AppState>>,
    request: QuickAnalyzeRequest,
) -> Result<QuickAnalyzeResponse, String> {
    // Rate limit: only 2 concurrent analyses allowed
    let _permit = state
        .analysis_semaphore
        .acquire()
        .await
        .map_err(|_| "Analysis rate limit exceeded".to_string())?;

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
pub async fn cmd_stream_analyze(
    window: tauri::Window,
    state: State<'_, Arc<AppState>>,
    request: AnalyzePositionRequest,
    on_token: tauri::ipc::Channel<StreamingTokenEvent>,
) -> Result<AnalyzePositionResponse, String> {
    // Rate limit: only 2 concurrent analyses allowed
    let _permit = state
        .analysis_semaphore
        .acquire()
        .await
        .map_err(|_| "Analysis rate limit exceeded".to_string())?;

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

// ─── Psychological Collapse Detection ───

#[tauri::command]
pub async fn cmd_detect_collapse(
    request: CollapseDetectRequest,
) -> Result<CollapseDetectResponse, String> {
    let collapses = crate::orchestrator::detect_collapse(&request.moves);
    Ok(CollapseDetectResponse { collapses })
}
