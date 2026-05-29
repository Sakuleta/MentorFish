// ─── Play Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;

use crate::database::DatabaseManager;
use crate::engine::EngineManager;

#[tauri::command]
pub async fn cmd_make_move(
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
pub async fn cmd_ai_move(
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

#[tauri::command]
pub async fn cmd_get_legal_moves(fen: String) -> Result<Vec<String>, String> {
    crate::engine::play::get_legal_moves(&fen).map_err(|e| format!("{}", e))
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
