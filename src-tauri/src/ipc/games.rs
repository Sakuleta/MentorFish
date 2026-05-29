// ─── Game Management Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;

use crate::database::GameStore;

#[tauri::command]
pub async fn cmd_save_game(
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
pub async fn cmd_get_game_moves(
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
pub async fn cmd_import_pgn(
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
pub async fn cmd_export_pgn(
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

    // Write to file if a path was provided (restricted to user's Documents directory)
    if let Some(ref path) = request.output_path {
        let resolved = std::path::PathBuf::from(path);
        // Ensure the path resolves to within the user's Documents folder
        let documents_dir = dirs_next::document_dir()
            .ok_or_else(|| "Cannot determine Documents directory".to_string())?;
        let canonical = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if !canonical.starts_with(&documents_dir) {
            return Err(format!(
                "Export path must be within your Documents folder ({}). Got: {}",
                documents_dir.display(),
                path
            ));
        }
        let content = pgn_output.clone();
        let write_path = canonical.clone();
        tokio::task::spawn_blocking(move || std::fs::write(&write_path, &content))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Failed to write PGN to '{}': {}", path, e))?;
    }

    Ok(ExportPgnResponse {
        pgn: pgn_output,
        game_count,
    })
}

#[tauri::command]
pub async fn cmd_get_recent_games(
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

// ─── Opening Explorer ───

#[tauri::command]
pub async fn cmd_get_opening(fen: String) -> Result<OpeningNodeResponse, String> {
    let node = crate::knowledge::retrieve_opening(&fen)
        .await
        .map_err(|e| format!("{}", e))?;
    Ok(OpeningNodeResponse { node })
}
