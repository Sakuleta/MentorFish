use crate::agents::UserProfile;
use crate::engine::EngineManager;
use crate::features::FeatureBundle;
use anyhow::Result;
use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position};

// ─── Legal Move Listing ───

pub fn get_legal_moves(fen: &str) -> Result<Vec<String>> {
    let fp: Fen = fen.parse()?;
    let pos: Chess = fp.into_position(CastlingMode::Standard)?;
    Ok(pos
        .legal_moves()
        .iter()
        .map(|m| UciMove::from_move(*m, CastlingMode::Standard).to_string())
        .collect())
}

use crate::engine::CandidateLine;

// ─── Strength Modes ───

/// How the engine selects its move in play mode.
#[derive(Debug, Clone)]
pub enum PlayStrength {
    /// Stockfish at maximum strength (default)
    FullStrength,
    /// Stockfish's built-in UCI_Elo limiting (1320..3190)
    StockfishElo(u32),
    /// Boltzmann-weighted candidate move selection for human-like errors
    Boltzmann { target_elo: u32 },
    /// Pedagogical move selection — biases toward positions rich in user weakness patterns
    Training,
}

// ─── Move Application Result ───

pub struct MoveResult {
    pub fen_after: String,
    pub is_check: bool,
    pub is_checkmate: bool,
    pub is_stalemate: bool,
    pub legal_moves_count: usize,
    pub ai_move: Option<String>,
    pub ai_fen: Option<String>,
}

// ─── Core Move Logic ───

pub fn apply_move(fen: &str, uci: &str) -> Result<MoveResult> {
    let fp: Fen = fen.parse()?;
    let pos: Chess = fp.into_position(CastlingMode::Standard)?;
    let mv = UciMove::from_ascii(uci.as_bytes())?.to_move(&pos)?;
    if !pos.is_legal(mv) {
        anyhow::bail!("Illegal move: {}", uci);
    }
    let new_pos = pos.play(mv)?;
    let fen_after = Fen::from_position(&new_pos, EnPassantMode::Legal).to_string();

    Ok(MoveResult {
        fen_after,
        is_check: new_pos.is_check(),
        is_checkmate: new_pos.is_checkmate(),
        is_stalemate: new_pos.is_stalemate(),
        legal_moves_count: new_pos.legal_moves().len(),
        ai_move: None,
        ai_fen: None,
    })
}

// ─── Human-Like Move Selection (Boltzmann) ───

/// Select a move from MultiPV candidates using Boltzmann weighting.
///
/// This produces human-plausible error patterns rather than random noise.
/// Candidates are expected to be sorted best-first (MultiPV order).
///
/// | Target ELO | Temperature T |
/// |-----------|---------------|
/// | ≤1200     | 180           |
/// | 1201–1500 | 120           |
/// | 1501–1800 | 70            |
/// | 1801–2000 | 40            |
/// | ≥2001     | 20            |
pub fn human_like_move(candidates: &[CandidateLine], target_elo: u32) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }

    let t = match target_elo {
        0..=1200 => 180.0,
        1201..=1500 => 120.0,
        1501..=1800 => 70.0,
        1801..=2000 => 40.0,
        _ => 20.0,
    };

    let best_eval = candidates[0].eval_cp.unwrap_or(0);
    let weights: Vec<f64> = candidates
        .iter()
        .map(|c| {
            let loss = (best_eval - c.eval_cp.unwrap_or(best_eval)) as f64;
            (-loss / t).exp()
        })
        .collect();

    let total: f64 = weights.iter().sum();
    let mut rng: f64 = rand::random();
    for (i, w) in weights.iter().enumerate() {
        rng -= w / total;
        if rng <= 0.0 {
            return candidates[i].pv.first().cloned();
        }
    }

    // Fallback to best move
    candidates[0].pv.first().cloned()
}

// ─── Play vs Engine ───

pub async fn play_vs_stockfish(
    engine: &dyn EngineManager,
    fen: &str,
    uci: Option<&str>,
    strength: &PlayStrength,
    user_profile: Option<&UserProfile>,
) -> Result<MoveResult> {
    let mut result = if let Some(uci) = uci {
        apply_move(fen, uci)?
    } else {
        // No user move — AI makes the opening move
        MoveResult {
            fen_after: fen.to_string(),
            is_check: false,
            is_checkmate: false,
            is_stalemate: false,
            legal_moves_count: 0,
            ai_move: None,
            ai_fen: None,
        }
    };
    if result.is_checkmate || result.is_stalemate {
        return Ok(result);
    }

    let fp: Fen = result.fen_after.parse()?;
    let pos: Chess = fp.into_position(CastlingMode::Standard)?;

    match strength {
        PlayStrength::FullStrength => {
            // Use depth 14 for responsive real-time play (depth 18+ can take 30+ s
            // with MultiPV enabled). The engine is configured with MultiPV which
            // makes each search slower, so keep depth moderate.
            let output = engine.analyze(&result.fen_after, Some(14), None).await?;
            apply_engine_move(&mut result, &pos, output.best_move.as_deref())?;
        }
        PlayStrength::StockfishElo(elo) => {
            // Enable built-in ELO limiting, then run analysis
            engine.configure_strength(Some(*elo)).await?;
            let output = engine.analyze(&result.fen_after, Some(18), None).await;
            // Always restore full strength, even on error
            engine.configure_strength(None).await.ok();
            let output = output?;
            apply_engine_move(&mut result, &pos, output.best_move.as_deref())?;
        }
        PlayStrength::Boltzmann { target_elo } => {
            // MultiPV=5 at depth 18, then Boltzmann-weighted selection
            // (MultiPV is already configured to 5 via engine config)
            let output = engine.analyze(&result.fen_after, Some(18), None).await?;
            let selected = human_like_move(&output.multipv, *target_elo);
            apply_engine_move(&mut result, &pos, selected.as_deref())?;
        }
        PlayStrength::Training => {
            // Pedagogical move selection biased toward user weakness patterns.
            // If no profile or no weaknesses, fall back to full-strength best move.
            if let Some(profile) = user_profile {
                let selected = training_mode_move(engine, &result.fen_after, profile).await?;
                apply_engine_move(&mut result, &pos, selected.as_deref())?;
            } else {
                let output = engine.analyze(&result.fen_after, None, None).await?;
                apply_engine_move(&mut result, &pos, output.best_move.as_deref())?;
            }
        }
    }

    Ok(result)
}

// ─── Training Mode: Pedagogical Move Selection ───

/// Select a pedagogically valuable move for training mode.
/// Favors positions rich in the user's weakness patterns via MultiPV=5 at depth 18.
pub async fn training_mode_move(
    engine: &dyn EngineManager,
    fen: &str,
    user_profile: &UserProfile,
) -> anyhow::Result<Option<String>> {
    // Get MultiPV analysis
    let output = engine.analyze(&fen.to_string(), Some(18), None).await?;

    if output.multipv.is_empty() {
        return Ok(output.best_move);
    }

    // For each candidate, project the resulting position and score it
    let mut scored_moves: Vec<(String, f64)> = Vec::new();

    for candidate in &output.multipv {
        if let Some(uci) = candidate.pv.first() {
            // Apply the move to get the resulting FEN
            if let Ok(new_fen) = project_move(fen, uci) {
                // Extract features from the resulting position
                if let Ok(features) = crate::features::extractor::extract_rule_based(&new_fen) {
                    let pedagogical_score = score_training_position(&features, user_profile);
                    let eval_score = candidate.eval_cp.unwrap_or(0) as f64 / 100.0;

                    // 70% pedagogical, 30% engine strength
                    let combined = 0.7 * pedagogical_score + 0.3 * eval_score.max(0.0);
                    scored_moves.push((uci.clone(), combined));
                }
            }
        }
    }

    // Select the move with highest combined score
    scored_moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(scored_moves.first().map(|(m, _)| m.clone()))
}

/// Project a move using shakmaty to get the resulting FEN.
fn project_move(fen: &str, uci: &str) -> anyhow::Result<String> {
    let fp: Fen = fen.parse()?;
    let pos: Chess = fp.into_position(CastlingMode::Standard)?;
    let mv = UciMove::from_ascii(uci.as_bytes())?.to_move(&pos)?;
    let new_pos = pos.play(mv)?;
    Ok(Fen::from_position(&new_pos, EnPassantMode::Legal).to_string())
}

/// Score a position based on how many features match user weakness patterns.
fn score_training_position(features: &FeatureBundle, profile: &UserProfile) -> f64 {
    let mut score = 0.0;

    for weakness in &profile.weakness_patterns {
        let pattern_lower = weakness.pattern_name.to_lowercase();

        // Check tactical features
        for tactical in &features.tactics {
            let tactical_str = format!("{:?}", tactical).to_lowercase();
            if tactical_str.contains(&pattern_lower) {
                score += weakness.occurrence_count as f64 * 0.5;
            }
        }

        // Check positional features
        for positional in &features.positional {
            let positional_str = format!("{:?}", positional).to_lowercase();
            if positional_str.contains(&pattern_lower) {
                score += weakness.occurrence_count as f64 * 0.3;
            }
        }
    }

    // Bonus: positions with high eval swing potential (dynamic positions are good for training)
    score += features.eval_swing_cp.abs() as f64 * 0.01;

    // Bonus: positions with tactical features (more to learn from)
    score += features.tactics.len() as f64 * 0.2;

    score
}

// ─── Apply Engine Move ───

/// Apply the engine's chosen move to produce ai-fields on the result.
fn apply_engine_move(result: &mut MoveResult, pos: &Chess, uci: Option<&str>) -> Result<()> {
    if let Some(bm) = uci {
        if let Ok(aim) = UciMove::from_ascii(bm.as_bytes())?.to_move(pos) {
            if pos.is_legal(aim) {
                let ai_pos = pos.clone().play(aim)?;
                result.ai_move = Some(bm.to_string());
                result.ai_fen = Some(Fen::from_position(&ai_pos, EnPassantMode::Legal).to_string());
                // Update terminal-state flags from the engine's move
                result.is_check = ai_pos.is_check();
                result.is_checkmate = ai_pos.is_checkmate();
                result.is_stalemate = ai_pos.is_stalemate();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn test_move_result_initial() {
        // Apply 1.e4 from the starting position
        let result = apply_move(START_FEN, "e2e4").unwrap();
        // After e4, the FEN should show the e4 pawn on rank 4
        assert!(!result.fen_after.is_empty());
        assert!(
            result.fen_after.contains("4P3"),
            "FEN after e4: {}",
            result.fen_after
        );
        assert!(!result.is_check);
        assert!(!result.is_checkmate);
        assert!(!result.is_stalemate);
        assert!(result.legal_moves_count > 0);
    }

    #[test]
    fn test_illegal_move() {
        // Ka1-a3 from start is illegal
        let result = apply_move(START_FEN, "a1a3");
        assert!(
            result.is_err(),
            "a1a3 should be illegal from start position"
        );
    }

    #[test]
    fn test_check_detection() {
        // Position: white queen on e2, black king on e8 — play Qe6 to give check
        let fen = "4k3/8/8/8/8/8/7Q/4K3 w - - 0 1";
        let result = apply_move(fen, "h2h8").unwrap();
        assert!(
            result.is_check,
            "Expected check after Qh8+, but is_check={}",
            result.is_check
        );
    }

    #[test]
    fn test_human_like_move_all_same() {
        // All candidates have the same eval — each should have equal weight
        let candidates = vec![
            CandidateLine {
                multipv: 1,
                pv: vec!["e2e4".to_string()],
                eval_cp: Some(30),
                eval_mate: None,
                depth: 10,
            },
            CandidateLine {
                multipv: 2,
                pv: vec!["d2d4".to_string()],
                eval_cp: Some(30),
                eval_mate: None,
                depth: 10,
            },
            CandidateLine {
                multipv: 3,
                pv: vec!["g1f3".to_string()],
                eval_cp: Some(30),
                eval_mate: None,
                depth: 10,
            },
        ];
        // With all same eval, any move can be selected (all have equal weight)
        let result = human_like_move(&candidates, 1500);
        assert!(
            result.is_some(),
            "Should return a move from non-empty candidates"
        );
        let mv = result.unwrap();
        assert!(
            mv == "e2e4" || mv == "d2d4" || mv == "g1f3",
            "Move should be one of the candidates, got: {}",
            mv
        );
    }

    #[test]
    fn test_human_like_move_empty() {
        let result = human_like_move(&[], 1500);
        assert!(result.is_none(), "Empty candidates should return None");
    }

    #[test]
    fn test_play_strength_default() {
        // Verify FullStrength is the first variant and constructible
        let strength = PlayStrength::FullStrength;
        match strength {
            PlayStrength::FullStrength => {} // expected
            _ => panic!("Expected FullStrength"),
        }
        // Also verify other variants
        let _elo = PlayStrength::StockfishElo(1800);
        let _boltz = PlayStrength::Boltzmann { target_elo: 1500 };
        let _training = PlayStrength::Training;
    }
}
