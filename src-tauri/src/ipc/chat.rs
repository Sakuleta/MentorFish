// ─── Conversational Chat Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;
use crate::orchestrator::PipelineCallbacks;

/// Send a chat message through the conversational coaching pipeline.
///
/// - If `fen` is provided, runs engine analysis and includes it as context.
/// - Streams tokens via Tauri events (`streaming-token`) for real-time UI updates.
/// - Returns the full reply text in the response.
#[tauri::command]
pub async fn cmd_chat_message(
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
pub async fn cmd_chat_message_stream(
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
