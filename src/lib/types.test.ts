import { describe, it, expect } from "vitest";
import type {
  FEN,
  UCIMove,
  Move,
  EngineOutput,
  CandidateLine,
  TacticalFeature,
  PositionalFeature,
  DynamicFeature,
  UserProfile,
  AnalyzePositionResponse,
  MakeMoveResponse,
  HealthCheckResponse,
  ChatMessageResponse,
  KnowledgeSummaryResponse,
  GameSummary,
} from "./types";

describe("types", () => {
  it("FEN type works", () => {
    const fen: FEN =
      "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    expect(fen).toContain("w KQkq");
  });

  it("UCIMove type works", () => {
    const uci: UCIMove = "e2e4";
    expect(uci).toBe("e2e4");
  });

  it("Move interface can be constructed", () => {
    const move: Move = {
      uci: "e2e4",
      san: "e4",
      move_number: 1,
      color: "White",
      fen_before:
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
      fen_after:
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
      eval_cp_before: 15,
      eval_cp_after: 25,
      eval_swing: 10,
      move_time_ms: 2500,
      classification: "Best",
    };
    expect(move.uci).toBe("e2e4");
    expect(move.color).toBe("White");
    expect(move.classification).toBe("Best");
  });

  it("EngineOutput interface can be constructed", () => {
    const output: EngineOutput = {
      fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
      eval_cp: 25,
      depth: 20,
      multipv: [],
    };
    expect(output.eval_cp).toBe(25);
    expect(output.depth).toBe(20);
  });

  it("CandidateLine interface can be constructed", () => {
    const line: CandidateLine = {
      multipv: 1,
      pv: ["e2e4", "e7e5", "g1f3"],
      eval_cp: 25,
      depth: 20,
    };
    expect(line.pv).toHaveLength(3);
  });

  it("TacticalFeature union works", () => {
    const fork: TacticalFeature = {
      Fork: { attacker_square: "e5", target_squares: ["c6", "g7"] },
    };
    expect("Fork" in fork).toBe(true);

    const pin: TacticalFeature = {
      Pin: {
        pinned_piece_square: "f3",
        pinner_square: "b7",
        shielded_piece_square: "d1",
        pin_type: "Relative",
      },
    };
    expect("Pin" in pin).toBe(true);
  });

  it("PositionalFeature union works", () => {
    const isolated: PositionalFeature = {
      IsolatedPawn: { square: "d4", color: "White" },
    };
    expect("IsolatedPawn" in isolated).toBe(true);
  });

  it("DynamicFeature union works", () => {
    const mobility: DynamicFeature = {
      PieceMobility: { square: "e4", legal_move_count: 8 },
    };
    expect("PieceMobility" in mobility).toBe(true);
  });

  it("UserProfile interface can be constructed", () => {
    const profile: UserProfile = {
      user_id: "test-user-id",
      tactical_accuracy: 0.75,
      positional_accuracy: 0.65,
      opening_knowledge: 0.8,
      endgame_technique: 0.5,
      time_management: 0.6,
      tilt_resistance: 0.9,
      style_profile: {},
      weakness_patterns: [],
      confidence: 0.5,
    };
    expect(profile.tactical_accuracy).toBe(0.75);
    expect(profile.weakness_patterns).toHaveLength(0);
  });

  it("AnalyzePositionResponse interface can be constructed", () => {
    const response: AnalyzePositionResponse = {
      explanation: {
        text: "Good move!",
        layer_breakdown: [],
        confidence: 0.85,
      },
      engine_eval: 35,
      best_move: "e2e4",
    };
    expect(response.engine_eval).toBe(35);
  });

  it("MakeMoveResponse interface can be constructed", () => {
    const response: MakeMoveResponse = {
      fen: "new-fen",
      isCheck: false,
      isCheckmate: false,
      isStalemate: false,
    };
    expect(response.isCheck).toBe(false);
  });

  it("HealthCheckResponse interface can be constructed", () => {
    const response: HealthCheckResponse = {
      engine_ok: true,
      inference_ok: true,
      database_ok: false,
    };
    expect(response.engine_ok).toBe(true);
  });

  it("ChatMessageResponse interface can be constructed", () => {
    const response: ChatMessageResponse = {
      reply: "Hello!",
    };
    expect(response.reply).toBe("Hello!");
  });

  it("KnowledgeSummaryResponse interface can be constructed", () => {
    const response: KnowledgeSummaryResponse = {
      total_books: 5,
      total_chunks: 120,
      total_embedded: 100,
      books: [],
    };
    expect(response.total_books).toBe(5);
  });

  it("GameSummary interface can be constructed", () => {
    const game: GameSummary = {
      game_id: "uuid-123",
      opponent: "Stockfish 16",
      result: "1-0",
      played_at: "2025-01-01",
      opening: "Sicilian Defense",
      move_count: 42,
    };
    expect(game.move_count).toBe(42);
  });
});
