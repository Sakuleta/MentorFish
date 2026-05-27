// ─── Feature Extractor ───
//
// Section 9.1 of the PRD: Rule-Based Extraction using shakmaty.
// Detects tactical, positional, and dynamic features from any FEN position.
// Zero LLM cost. Pure computation.

use anyhow::Result;
use shakmaty::fen::Fen;
use shakmaty::{
    attacks, Bitboard, Board, CastlingMode, Chess, Color as ShakColor, File, Position, Rank, Role,
    Square,
};

use super::{
    CandidateMove, DynamicFeature, ExtractionConfidence, FeatureBundle, PinType, PositionalFeature,
    TacticalFeature,
};
use crate::engine::{EngineManager, EngineOutput};

/// Full extraction: rule-based features + Stockfish integration.
pub async fn extract(
    fen: &str,
    engine: Option<&dyn EngineManager>,
    prev_eval_cp: Option<i32>,
) -> Result<FeatureBundle> {
    let pos = parse_fen(fen)?;
    let board = pos.board();
    let occupied = board.occupied();

    let tactics = detect_tactics(&pos, board, occupied);
    let positional = detect_positional(&pos, board);
    let dynamic = detect_dynamic(&pos, board, occupied);

    let (eval_cp, is_forced_mate, mate_in, top_moves, eval_swing_cp) = if let Some(engine) = engine
    {
        match engine.analyze(&fen.to_string(), None, None).await {
            Ok(output) => {
                let moves = convert_candidate_moves(&output);
                let swing = prev_eval_cp.map(|prev| output.eval_cp - prev).unwrap_or(0);
                (
                    output.eval_cp,
                    output.eval_mate.is_some(),
                    output.eval_mate,
                    moves,
                    swing,
                )
            }
            Err(_) => (0, false, None, vec![], 0),
        }
    } else {
        (0, false, None, vec![], 0)
    };

    Ok(FeatureBundle {
        position_fen: fen.to_string(),
        eval_cp,
        eval_swing_cp,
        is_forced_mate,
        mate_in,
        top_moves,
        tactics,
        positional,
        dynamic,
        confidence: ExtractionConfidence::High,
    })
}

/// Rule-based only (no engine call) — for testing and fast paths.
pub fn extract_rule_based(fen: &str) -> Result<FeatureBundle> {
    let pos = parse_fen(fen)?;
    let board = pos.board();
    let occupied = board.occupied();

    let tactics = detect_tactics(&pos, board, occupied);
    let positional = detect_positional(&pos, board);
    let dynamic = detect_dynamic(&pos, board, occupied);

    Ok(FeatureBundle {
        position_fen: fen.to_string(),
        eval_cp: 0,
        eval_swing_cp: 0,
        is_forced_mate: false,
        mate_in: None,
        top_moves: vec![],
        tactics,
        positional,
        dynamic,
        confidence: ExtractionConfidence::High,
    })
}

fn parse_fen(fen: &str) -> Result<Chess> {
    let parsed: Fen = fen
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))?;
    parsed
        .into_position(CastlingMode::Standard)
        .map_err(|e| anyhow::anyhow!("Invalid position: {}", e))
}

fn convert_candidate_moves(output: &EngineOutput) -> Vec<CandidateMove> {
    output
        .multipv
        .iter()
        .map(|line| CandidateMove {
            uci: line.pv.first().cloned().unwrap_or_default(),
            san: None,
            eval_cp: line.eval_cp,
            mate_in: line.eval_mate,
            eval_loss_cp: line.eval_cp.map(|cp| output.eval_cp - cp),
            pv: line.pv.clone(),
            depth: line.depth,
        })
        .collect()
}

// ═══════════════════════════════════════════
//  SHARED HELPERS
// ═══════════════════════════════════════════

fn file_prev(f: File) -> Option<File> {
    let idx = f as i32 - 1;
    if idx >= 0 {
        Some(File::new(idx as u32))
    } else {
        None
    }
}

fn file_next(f: File) -> Option<File> {
    let idx = f as i32 + 1;
    if idx < 8 {
        Some(File::new(idx as u32))
    } else {
        None
    }
}

fn pieces_by_color(board: &Board, color: ShakColor, role: Role) -> Bitboard {
    let all_of_role = match role {
        Role::Pawn => board.pawns(),
        Role::Knight => board.knights(),
        Role::Bishop => board.bishops(),
        Role::Rook => board.rooks(),
        Role::Queen => board.queens(),
        Role::King => board.kings(),
    };
    all_of_role & board.by_color(color)
}

fn piece_value(role: Role) -> u32 {
    match role {
        Role::Pawn => 1,
        Role::Knight => 3,
        Role::Bishop => 3,
        Role::Rook => 5,
        Role::Queen => 9,
        Role::King => 100,
    }
}

// ═══════════════════════════════════════════
//  TACTICAL FEATURE DETECTION
// ═══════════════════════════════════════════

fn detect_tactics(pos: &Chess, board: &Board, occupied: Bitboard) -> Vec<TacticalFeature> {
    let mut features = Vec::new();
    let turn = pos.turn();
    features.extend(detect_hanging_pieces(board, occupied, turn));
    features.extend(detect_forks(board, occupied, turn));
    features.extend(detect_pins_and_skewers(board, occupied, turn));
    features.extend(detect_discovered_attacks(board, occupied, turn));
    features
}

fn detect_hanging_pieces(
    board: &Board,
    occupied: Bitboard,
    stm: ShakColor,
) -> Vec<TacticalFeature> {
    let mut features = Vec::new();
    let defender_color = !stm;
    for sq in occupied {
        if let Some(piece) = board.piece_at(sq) {
            if piece.color == defender_color {
                let attackers = board.attacks_to(sq, stm, occupied).count();
                let defenders = board.attacks_to(sq, defender_color, occupied).count();
                if attackers > defenders {
                    features.push(TacticalFeature::HangingPiece {
                        square: sq.to_string(),
                        piece_type: format!("{:?}", piece.role),
                    });
                }
            }
        }
    }
    features
}

fn detect_forks(board: &Board, occupied: Bitboard, stm: ShakColor) -> Vec<TacticalFeature> {
    let mut features = Vec::new();
    let opp = !stm;
    for sq in occupied {
        if let Some(piece) = board.piece_at(sq) {
            if piece.color != stm {
                continue;
            }
            let attacks = board.attacks_from(sq);
            let mut targets: Vec<String> = Vec::new();
            for target_sq in attacks {
                if let Some(target) = board.piece_at(target_sq) {
                    if target.color == opp {
                        let defs = board.attacks_to(target_sq, opp, occupied).count();
                        if defs == 0 || piece_value(target.role) > piece_value(piece.role) {
                            targets.push(target_sq.to_string());
                        }
                    }
                }
            }
            if targets.len() >= 2 {
                features.push(TacticalFeature::Fork {
                    attacker_square: sq.to_string(),
                    target_squares: targets,
                });
            }
        }
    }
    features
}

fn detect_pins_and_skewers(
    board: &Board,
    occupied: Bitboard,
    stm: ShakColor,
) -> Vec<TacticalFeature> {
    let mut features = Vec::new();
    let opp = !stm;
    for sq in occupied {
        if let Some(piece) = board.piece_at(sq) {
            if piece.color != opp {
                continue;
            }
            if !matches!(piece.role, Role::Bishop | Role::Rook | Role::Queen) {
                continue;
            }
            let attacks = attacks::attacks(sq, piece, occupied);
            for target_sq in attacks {
                if let Some(target) = board.piece_at(target_sq) {
                    if target.color != stm {
                        continue;
                    }
                    if attacks::aligned(sq, target_sq, sq) {
                        let ray = attacks::ray(sq, target_sq);
                        for behind_sq in ray & occupied {
                            if behind_sq == target_sq {
                                continue;
                            }
                            if let Some(behind) = board.piece_at(behind_sq) {
                                if behind.color == stm {
                                    let ptype = if behind.role == Role::King {
                                        PinType::Absolute
                                    } else if piece_value(behind.role) > piece_value(target.role) {
                                        PinType::Relative
                                    } else {
                                        continue;
                                    };
                                    features.push(TacticalFeature::Pin {
                                        pinned_piece_square: target_sq.to_string(),
                                        pinner_square: sq.to_string(),
                                        shielded_piece_square: behind_sq.to_string(),
                                        pin_type: ptype,
                                    });
                                } else if behind.color == opp
                                    && piece_value(behind.role) > piece_value(target.role)
                                {
                                    features.push(TacticalFeature::Skewer {
                                        skewered_piece_square: target_sq.to_string(),
                                        attacker_square: sq.to_string(),
                                        shielded_piece_square: behind_sq.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    features
}

fn detect_discovered_attacks(
    board: &Board,
    occupied: Bitboard,
    stm: ShakColor,
) -> Vec<TacticalFeature> {
    let mut features = Vec::new();
    let opp = !stm;
    for sq in occupied {
        if let Some(piece) = board.piece_at(sq) {
            if piece.color != stm {
                continue;
            }
            let behind_bb =
                attacks::rook_attacks(sq, occupied) | attacks::bishop_attacks(sq, occupied);
            for behind_sq in behind_bb & occupied {
                if let Some(behind) = board.piece_at(behind_sq) {
                    if behind.color != stm {
                        continue;
                    }
                    if !matches!(behind.role, Role::Bishop | Role::Rook | Role::Queen) {
                        continue;
                    }
                    let occ_without = occupied.without(sq);
                    let revealed = attacks::attacks(behind_sq, behind, occ_without);
                    for target_sq in revealed & occupied {
                        if let Some(target) = board.piece_at(target_sq) {
                            if target.color == opp {
                                let already = attacks::attacks(behind_sq, behind, occupied)
                                    .contains(target_sq);
                                if !already {
                                    features.push(TacticalFeature::DiscoveredAttack {
                                        mover_square: sq.to_string(),
                                        revealed_attacker_square: behind_sq.to_string(),
                                        target_square: target_sq.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    features
}

// ═══════════════════════════════════════════
//  POSITIONAL FEATURE DETECTION
// ═══════════════════════════════════════════

fn detect_positional(pos: &Chess, board: &Board) -> Vec<PositionalFeature> {
    let mut features = Vec::new();
    features.extend(pawn_features(pos, board));
    features.extend(outpost_features(pos, board));
    features.extend(file_features(pos, board));
    features.extend(bishop_pair_features(pos, board));
    features.extend(king_safety_features(pos, board));
    features
}

fn pawn_features(_pos: &Chess, board: &Board) -> Vec<PositionalFeature> {
    let mut features = Vec::new();
    for color in [ShakColor::White, ShakColor::Black] {
        let color_str = format!("{:?}", color).to_lowercase();
        let my_pawns = pieces_by_color(board, color, Role::Pawn);
        let opp_pawns = pieces_by_color(board, !color, Role::Pawn);
        let mut file_counts = [0u8; 8];
        for sq in my_pawns {
            let file = sq.file();
            file_counts[file as usize] += 1;

            // Isolated pawn
            let has_left =
                file_prev(file).is_some_and(|f| (my_pawns & Bitboard::from_file(f)).any());
            let has_right =
                file_next(file).is_some_and(|f| (my_pawns & Bitboard::from_file(f)).any());
            if !has_left && !has_right {
                features.push(PositionalFeature::IsolatedPawn {
                    square: sq.to_string(),
                    color: color_str.clone(),
                });
            }

            // Passed pawn
            let advance_dir: i32 = if color == ShakColor::White { 1 } else { -1 };
            let mut passed = true;
            for check_file in [file_prev(file), Some(file), file_next(file)] {
                if let Some(cf) = check_file {
                    for ro in 1..=7 {
                        let target_rank_val = (sq.rank() as i32 + ro * advance_dir) as u32;
                        if target_rank_val >= 8 {
                            break;
                        }
                        if (opp_pawns & Bitboard::from_file(cf))
                            .into_iter()
                            .any(|s| s.rank() as u32 == target_rank_val)
                        {
                            passed = false;
                            break;
                        }
                    }
                }
                if !passed {
                    break;
                }
            }
            if passed {
                features.push(PositionalFeature::PassedPawn {
                    square: sq.to_string(),
                    color: color_str.clone(),
                });
            }
        }

        // Doubled pawns
        for (fi, &cnt) in file_counts.iter().enumerate() {
            if cnt >= 2 {
                let file = File::new(fi as u32);
                features.push(PositionalFeature::DoubledPawn {
                    file: file.to_string(),
                    color: color_str.clone(),
                });
            }
        }

        // Backward pawns
        for sq in my_pawns {
            let file = sq.file();
            if (opp_pawns & Bitboard::from_file(file)).any() {
                continue;
            }
            let adv: i32 = if color == ShakColor::White { 1 } else { -1 };
            let cur_r = sq.rank() as i32;
            let left_ahead = file_prev(file).is_some_and(|f| {
                (my_pawns & Bitboard::from_file(f))
                    .into_iter()
                    .any(|s| (s.rank() as i32 - cur_r) * adv > 0)
            });
            let right_ahead = file_next(file).is_some_and(|f| {
                (my_pawns & Bitboard::from_file(f))
                    .into_iter()
                    .any(|s| (s.rank() as i32 - cur_r) * adv > 0)
            });
            if !left_ahead && !right_ahead {
                features.push(PositionalFeature::BackwardPawn {
                    square: sq.to_string(),
                    color: color_str.clone(),
                });
            }
        }
    }
    features
}

fn outpost_features(_pos: &Chess, board: &Board) -> Vec<PositionalFeature> {
    let mut features = Vec::new();
    for color in [ShakColor::White, ShakColor::Black] {
        let color_str = format!("{:?}", color).to_lowercase();
        let opp_pawns = pieces_by_color(board, !color, Role::Pawn);
        let my_knights = pieces_by_color(board, color, Role::Knight);
        let my_bishops = pieces_by_color(board, color, Role::Bishop);
        let minors = my_knights | my_bishops;

        let territory: Vec<Rank> = if color == ShakColor::White {
            vec![Rank::Fifth, Rank::Sixth, Rank::Seventh, Rank::Eighth]
        } else {
            vec![Rank::First, Rank::Second, Rank::Third, Rank::Fourth]
        };

        for rank in territory {
            for file_idx in 0..8u32 {
                let file = File::new(file_idx);
                let sq = Square::from_coords(file, rank);
                if opp_pawns.contains(sq) {
                    continue;
                }
                let pa = attacks::pawn_attacks(!color, sq);
                if !(pa & opp_pawns).is_empty() {
                    continue;
                }
                if minors.contains(sq) {
                    features.push(PositionalFeature::Outpost {
                        square: sq.to_string(),
                        color: color_str.clone(),
                    });
                }
            }
        }
    }
    features
}

fn file_features(_pos: &Chess, board: &Board) -> Vec<PositionalFeature> {
    let mut features = Vec::new();
    let w_pawns = pieces_by_color(board, ShakColor::White, Role::Pawn);
    let b_pawns = pieces_by_color(board, ShakColor::Black, Role::Pawn);
    for file_idx in 0..8u32 {
        let file = File::new(file_idx);
        let has_w = (w_pawns & Bitboard::from_file(file)).any();
        let has_b = (b_pawns & Bitboard::from_file(file)).any();
        if !has_w && !has_b {
            features.push(PositionalFeature::OpenFile {
                file: file.to_string(),
            });
        } else if !has_w {
            features.push(PositionalFeature::HalfOpenFile {
                file: file.to_string(),
                color: "white".into(),
            });
        } else if !has_b {
            features.push(PositionalFeature::HalfOpenFile {
                file: file.to_string(),
                color: "black".into(),
            });
        }
    }
    features
}

fn bishop_pair_features(_pos: &Chess, board: &Board) -> Vec<PositionalFeature> {
    let mut features = Vec::new();
    for color in [ShakColor::White, ShakColor::Black] {
        if pieces_by_color(board, color, Role::Bishop).count() >= 2 {
            features.push(PositionalFeature::BishopPair {
                color: format!("{:?}", color).to_lowercase(),
            });
        }
    }
    features
}

fn king_safety_features(_pos: &Chess, board: &Board) -> Vec<PositionalFeature> {
    let mut features = Vec::new();
    for color in [ShakColor::White, ShakColor::Black] {
        let color_str = format!("{:?}", color).to_lowercase();
        if let Some(king_sq) = board.king_of(color) {
            let kf = king_sq.file();
            let kr = king_sq.rank();
            let adv: i32 = if color == ShakColor::White { 1 } else { -1 };
            // Clamp shield rank to valid range [0,7] — the king may be on
            // the back rank (e.g. after kingside castling) where advancing
            // one rank would overflow.
            let shield_rank = (kr as i32 + adv).clamp(0, 7) as u32;

            let mut shield = 0u32;
            let sr = Rank::new(shield_rank);
            for f in [file_prev(kf), Some(kf), file_next(kf)]
                .into_iter()
                .flatten()
            {
                let sq = Square::from_coords(f, sr);
                if let Some(p) = board.piece_at(sq) {
                    if p.color == color && p.role == Role::Pawn {
                        shield += 1;
                    }
                }
            }
            let shield_ratio = shield as f64 / 3.0;

            let mut open_near = 0u32;
            for off in -2i32..=2i32 {
                let fi = kf as i32 + off;
                // Skip out-of-range files when king is near the edge
                if !(0..=7).contains(&fi) {
                    continue;
                }
                let cf = File::new(fi as u32);
                let fp = pieces_by_color(board, color, Role::Pawn) & Bitboard::from_file(cf);
                if fp.is_empty() {
                    open_near += 1;
                }
            }
            features.push(PositionalFeature::KingSafety {
                color: color_str,
                pawn_shield_completeness: shield_ratio,
                open_files_near_king: open_near,
            });
        }
    }
    features
}

// ═══════════════════════════════════════════
//  DYNAMIC FEATURE DETECTION
// ═══════════════════════════════════════════

fn detect_dynamic(pos: &Chess, board: &Board, occupied: Bitboard) -> Vec<DynamicFeature> {
    let mut features = Vec::new();
    let turn = pos.turn();
    let legal_moves = pos.legal_moves();

    // Piece mobility
    let mut mobile: std::collections::HashMap<Square, u32> = std::collections::HashMap::new();
    for m in &legal_moves {
        *mobile.entry(m.from().unwrap_or(Square::A1)).or_default() += 1;
    }
    for (sq, cnt) in mobile {
        if cnt > 0 {
            features.push(DynamicFeature::PieceMobility {
                square: sq.to_string(),
                legal_move_count: cnt,
            });
        }
    }

    // Space advantage
    let territory: Vec<Rank> = if turn == ShakColor::White {
        vec![Rank::Fifth, Rank::Sixth, Rank::Seventh, Rank::Eighth]
    } else {
        vec![Rank::First, Rank::Second, Rank::Third, Rank::Fourth]
    };
    let mut controlled = 0u32;
    for rank in territory {
        for fi in 0..8u32 {
            let file = File::new(fi);
            let sq = Square::from_coords(file, rank);
            if board.attacks_to(sq, turn, occupied).any() {
                controlled += 1;
            }
        }
    }
    features.push(DynamicFeature::SpaceAdvantage {
        color: format!("{:?}", turn).to_lowercase(),
        controlled_squares: controlled,
    });

    // Development
    let back_rank = if turn == ShakColor::White {
        Rank::First
    } else {
        Rank::Eighth
    };
    let back_bb = Bitboard::from_rank(back_rank);
    let knights = pieces_by_color(board, turn, Role::Knight);
    let bishops = pieces_by_color(board, turn, Role::Bishop);
    let dev_knights = (knights & !back_bb).count() as u32;
    let dev_bishops = (bishops & !back_bb).count() as u32;
    features.push(DynamicFeature::Development {
        color: format!("{:?}", turn).to_lowercase(),
        minor_pieces_developed: dev_knights + dev_bishops,
    });

    // Initiative: threats
    let opp = !turn;
    let mut threats = 0u32;
    for sq in occupied {
        if let Some(p) = board.piece_at(sq) {
            if p.color == opp && board.attacks_to(sq, turn, occupied).any() {
                threats += 1;
            }
        }
    }
    features.push(DynamicFeature::Initiative {
        color: format!("{:?}", turn).to_lowercase(),
        threats_count: threats,
    });

    features
}

// ═══════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn count_positional(features: &[PositionalFeature], name: &str) -> usize {
        features
            .iter()
            .filter(|f| match f {
                PositionalFeature::IsolatedPawn { .. } => name == "IsolatedPawn",
                PositionalFeature::DoubledPawn { .. } => name == "DoubledPawn",
                PositionalFeature::BackwardPawn { .. } => name == "BackwardPawn",
                PositionalFeature::PassedPawn { .. } => name == "PassedPawn",
                PositionalFeature::Outpost { .. } => name == "Outpost",
                PositionalFeature::OpenFile { .. } => name == "OpenFile",
                PositionalFeature::HalfOpenFile { .. } => name == "HalfOpenFile",
                PositionalFeature::BishopPair { .. } => name == "BishopPair",
                PositionalFeature::KingSafety { .. } => name == "KingSafety",
                _ => false,
            })
            .count()
    }

    #[test]
    fn test_initial_position() {
        let bundle =
            extract_rule_based("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(count_positional(&bundle.positional, "IsolatedPawn"), 0);
        assert_eq!(count_positional(&bundle.positional, "DoubledPawn"), 0);
        assert_eq!(count_positional(&bundle.positional, "PassedPawn"), 0);
        assert_eq!(count_positional(&bundle.positional, "OpenFile"), 0);
        assert_eq!(count_positional(&bundle.positional, "BishopPair"), 2);
    }

    #[test]
    fn test_isolated_pawn() {
        // Two kings + a white pawn on c4, no white pawns on b or d files
        let fen = "4k3/8/8/3p4/2P5/8/8/4K3 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        assert!(
            count_positional(&bundle.positional, "IsolatedPawn") >= 1,
            "Expected isolated pawn in: {:?}",
            bundle.positional
        );
    }

    #[test]
    fn test_doubled_pawns() {
        // After 1.e4 f5 2.exf5 — Black doubled f-pawns (f7+f5)
        let fen = "rnbqkbnr/ppppp1pp/8/5P2/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2";
        let bundle = extract_rule_based(fen).unwrap();
        let doubled = bundle
            .positional
            .iter()
            .filter(|f| matches!(f, PositionalFeature::DoubledPawn { .. }))
            .count();
        assert!(
            doubled >= 1,
            "Expected doubled pawns in: {:?}",
            bundle.positional
        );
    }

    #[test]
    fn test_open_file() {
        // Both d-pawns missing from standard start — d-file is open
        let fen = "rnbqkbnr/ppp1pppp/8/8/8/8/PPP1PPPP/RNBQKBNR w KQkq - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        assert!(
            count_positional(&bundle.positional, "OpenFile") >= 1,
            "Expected open file in: {:?}",
            bundle.positional
        );
    }

    #[test]
    fn test_passed_pawn() {
        // Two kings + white pawn on e6; no black pawns on d/e/f files
        let fen = "4k3/8/4P3/8/8/8/8/4K3 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        assert!(
            count_positional(&bundle.positional, "PassedPawn") >= 1,
            "Expected passed pawn in: {:?}",
            bundle.positional
        );
    }

    #[test]
    fn test_tactics_detected() {
        let bundle =
            extract_rule_based("rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq - 0 2")
                .unwrap();
        assert!(!bundle.tactics.is_empty() || !bundle.positional.is_empty());
    }

    #[test]
    fn test_bishop_pair() {
        let bundle =
            extract_rule_based("r1bqkbnr/pppp1ppp/2n5/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 3")
                .unwrap();
        assert!(count_positional(&bundle.positional, "BishopPair") >= 1);
    }

    #[test]
    fn test_dynamic_features() {
        let bundle =
            extract_rule_based("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert!(!bundle.dynamic.is_empty());
        let has_mob = bundle
            .dynamic
            .iter()
            .any(|d| matches!(d, DynamicFeature::PieceMobility { .. }));
        let has_dev = bundle
            .dynamic
            .iter()
            .any(|d| matches!(d, DynamicFeature::Development { .. }));
        assert!(has_mob || has_dev);
    }

    // ── Tactical feature detection tests ──

    #[test]
    fn test_fork_detection() {
        // White knight on e4 forks black rooks on d6 and f6
        let fen = "4k3/8/3r1r2/8/4N3/8/8/4K3 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        let forks: Vec<_> = bundle
            .tactics
            .iter()
            .filter(|t| matches!(t, TacticalFeature::Fork { .. }))
            .collect();
        assert!(
            !forks.is_empty(),
            "Expected fork in position {}, tactics: {:?}",
            fen,
            bundle.tactics
        );
    }

    #[test]
    fn test_pin_detection() {
        // Black rook on e8 pins white knight on e2 to white king on e1 (absolute pin)
        let fen = "3kr3/8/8/8/8/8/4N3/4K3 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        let pins: Vec<_> = bundle
            .tactics
            .iter()
            .filter(|t| matches!(t, TacticalFeature::Pin { .. }))
            .collect();
        assert!(
            !pins.is_empty(),
            "Expected pin in position {}, tactics: {:?}",
            fen,
            bundle.tactics
        );
        // Verify it's an absolute pin
        let has_absolute = pins.iter().any(|p| {
            if let TacticalFeature::Pin { pin_type, .. } = p {
                matches!(pin_type, PinType::Absolute)
            } else {
                false
            }
        });
        assert!(has_absolute, "Expected absolute pin, got: {:?}", pins);
    }

    #[test]
    fn test_skewer_detection() {
        // Black rook on a8 skewers white knight on a4, with black queen behind on a1
        let fen = "r4k2/8/8/8/N7/8/8/q2K4 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        let skewers: Vec<_> = bundle
            .tactics
            .iter()
            .filter(|t| matches!(t, TacticalFeature::Skewer { .. }))
            .collect();
        assert!(
            !skewers.is_empty(),
            "Expected skewer in position {}, tactics: {:?}",
            fen,
            bundle.tactics
        );
    }

    #[test]
    fn test_hanging_piece() {
        // e4 pawn attacks undefended d5 pawn — simple two-king position
        let fen = "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        let hanging: Vec<_> = bundle
            .tactics
            .iter()
            .filter(|t| matches!(t, TacticalFeature::HangingPiece { .. }))
            .collect();
        assert!(
            !hanging.is_empty(),
            "Expected hanging piece in position {}, tactics: {:?}",
            fen,
            bundle.tactics
        );
    }

    #[test]
    fn test_discovered_attack() {
        // White knight on a4 blocks white rook on a2 from attacking black queen on a7
        let fen = "4k3/q7/8/8/N7/8/R7/4K3 w - - 0 1";
        let bundle = extract_rule_based(fen).unwrap();
        let discovered: Vec<_> = bundle
            .tactics
            .iter()
            .filter(|t| matches!(t, TacticalFeature::DiscoveredAttack { .. }))
            .collect();
        assert!(
            !discovered.is_empty(),
            "Expected discovered attack in position {}, tactics: {:?}",
            fen,
            bundle.tactics
        );
    }

    #[test]
    fn test_no_false_positives() {
        // Initial position should have no forks, pins, or skewers
        let bundle =
            extract_rule_based("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let tactical: Vec<_> = bundle
            .tactics
            .iter()
            .filter(|t| {
                matches!(
                    t,
                    TacticalFeature::Fork { .. }
                        | TacticalFeature::Pin { .. }
                        | TacticalFeature::Skewer { .. }
                )
            })
            .collect();
        assert!(
            tactical.is_empty(),
            "Initial position should have no forks/pins/skewers, got: {:?}",
            tactical
        );
    }
}
