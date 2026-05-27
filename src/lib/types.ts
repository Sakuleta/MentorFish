// ─── Frontend Type Definitions ───
//
// Mirrors the Rust types in src-tauri/src/
// These are the types received from Tauri IPC commands.

// ─── Plan Visualization ───

export interface PlanMove {
  uci: string;
  description: string;
}

// ─── Core Chess Types ───

export type FEN = string;
export type UCIMove = string;

export interface Move {
  uci: UCIMove;
  san?: string;
  move_number: number;
  color: Color;
  fen_before: FEN;
  fen_after: FEN;
  eval_cp_before?: number;
  eval_cp_after?: number;
  eval_swing?: number;
  move_time_ms?: number;
  classification?: MoveClassification;
}

export type Color = "White" | "Black";

export type MoveClassification =
  | "Best"
  | "Good"
  | "Inaccuracy"
  | "Mistake"
  | "Blunder";

// ─── Engine Output ───

export interface EngineOutput {
  fen: FEN;
  eval_cp: number;
  eval_mate?: number;
  best_move?: UCIMove;
  best_move_san?: string;
  ponder?: UCIMove;
  depth: number;
  multipv: CandidateLine[];
  nodes?: number;
  nps?: number;
  time_ms?: number;
}

export interface CandidateLine {
  multipv: number;
  pv: UCIMove[];
  eval_cp?: number;
  eval_mate?: number;
  depth: number;
}

// ─── Feature Bundle ───

export interface FeatureBundle {
  position_fen: FEN;
  eval_cp: number;
  eval_swing_cp: number;
  is_forced_mate: boolean;
  mate_in?: number;
  top_moves: CandidateMove[];
  tactics: TacticalFeature[];
  positional: PositionalFeature[];
  dynamic: DynamicFeature[];
  confidence: "High" | "Medium";
}

export interface CandidateMove {
  uci: UCIMove;
  san?: string;
  eval_cp?: number;
  mate_in?: number;
  eval_loss_cp?: number;
  pv: UCIMove[];
  depth: number;
}

// ─── Tactical Features ───

export type TacticalFeature =
  | { Fork: { attacker_square: string; target_squares: string[] } }
  | {
      Pin: {
        pinned_piece_square: string;
        pinner_square: string;
        shielded_piece_square: string;
        pin_type: "Absolute" | "Relative";
      };
    }
  | {
      Skewer: {
        skewered_piece_square: string;
        attacker_square: string;
        shielded_piece_square: string;
      };
    }
  | { HangingPiece: { square: string; piece_type: string } }
  | {
      DiscoveredAttack: {
        mover_square: string;
        revealed_attacker_square: string;
        target_square: string;
      };
    };

// ─── Positional Features ───

export type PositionalFeature =
  | { IsolatedPawn: { square: string; color: string } }
  | { DoubledPawn: { file: string; color: string } }
  | { BackwardPawn: { square: string; color: string } }
  | { PassedPawn: { square: string; color: string } }
  | { Outpost: { square: string; color: string } }
  | { OpenFile: { file: string } }
  | { HalfOpenFile: { file: string; color: string } }
  | { BishopPair: { color: string } }
  | { PawnIsland: { color: string; count: number } }
  | {
      KingSafety: {
        color: string;
        pawn_shield_completeness: number;
        open_files_near_king: number;
      };
    };

// ─── Dynamic Features ───

export type DynamicFeature =
  | { PieceMobility: { square: string; legal_move_count: number } }
  | { SpaceAdvantage: { color: string; controlled_squares: number } }
  | { Development: { color: string; minor_pieces_developed: number } }
  | { Initiative: { color: string; threats_count: number } };

// ─── Agent Outputs ───

export interface TacticalSummary {
  blunders: BlunderRecord[];
  missed_tactics: TacticRecord[];
  eval_swings: EvalSwing[];
  forcing_sequences: ForcingLine[];
  confidence: number;
}

export interface BlunderRecord {
  uci_move: UCIMove;
  eval_swing_cp: number;
  position_fen: string;
  description: string;
}

export interface TacticRecord {
  uci_opportunity: UCIMove;
  eval_improvement_cp: number;
  tactic_type: string;
  position_fen: string;
}

export interface EvalSwing {
  move_number: number;
  swing_cp: number;
  from_eval: number;
  to_eval: number;
}

export interface ForcingLine {
  uci_sequence: UCIMove[];
  eval_result_cp: number;
  classification: string;
}

// ─── Explanation ───

export interface FinalExplanation {
  text: string;
  layer_breakdown: LayerContent[];
  confidence: number;
  low_confidence_note?: string;
}

export interface LayerContent {
  layer: number;
  layer_name: string;
  content: string;
  confidence: number;
}

// ─── User Profile ───

export interface UserProfile {
  user_id: string;
  tactical_accuracy: number;
  positional_accuracy: number;
  opening_knowledge: number;
  endgame_technique: number;
  time_management: number;
  tilt_resistance: number;
  style_profile: Record<string, unknown>;
  weakness_patterns: WeaknessPattern[];
  confidence: number;
}

export interface WeaknessPattern {
  id: string;
  pattern_name: string;
  description?: string;
  occurrence_count: number;
  last_seen?: string;
}

// ─── Opening Explorer Types ───

export interface OpeningMove {
  uci: UCIMove;
  san: string;
  frequency: number;
}

export interface OpeningNode {
  fen: FEN;
  eco?: string;
  opening_name?: string;
  frequency?: number;
  white_score?: number;
  children: OpeningMove[];
}

export interface OpeningNodeResponse {
  node: OpeningNode | null;
}

// ─── IPC Response Types ───

export interface AnalyzePositionResponse {
  explanation: FinalExplanation;
  engine_eval: number;
  best_move?: string;
}

export interface HealthCheckResponse {
  engine_ok: boolean;
  inference_ok: boolean;
  database_ok: boolean;
}

// ─── Play Move Types ───

export interface MakeMoveRequest {
  fen: string;
  uci: string;
  vsAi: boolean;
  strengthMode?: string;
  targetElo?: number;
}

export interface MakeMoveResponse {
  fen: string;
  isCheck: boolean;
  isCheckmate: boolean;
  isStalemate: boolean;
  aiMove?: string;
  aiFen?: string;
}

// ─── Chat / Conversational Types ───

export interface ChatHistoryEntry {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface ChatMessageRequest {
  message: string;
  fen?: string;
  history: ChatHistoryEntry[];
  persona?: string;
}

export interface ChatMessageResponse {
  reply: string;
}

// ─── Curriculum / Study Plan ───

export interface StudyPlan {
  weekly_sessions: StudySession[];
  opening_drills: OpeningDrill[];
  endgame_exercises: EndgameExercise[];
  tactical_puzzle_theme: string;
  rationale: string;
}

export interface StudySession {
  day: string;
  focus: string;
  duration_minutes: number;
  description: string;
}

export interface OpeningDrill {
  opening_name: string;
  color: string;
  focus: string;
}

export interface EndgameExercise {
  exercise_type: string;
  description: string;
  position_fen?: string;
}

// ─── Tauri Events ───

export interface StreamingTokenEvent {
  token: string;
  is_final: boolean;
}

export interface CoachingAlertEvent {
  alert_type: string;
  message: string;
  position_fen: string;
}

export interface CoachingTriggerEvent {
  triggerType: string;
  message: string;
  severity: string;
  positionFen: string;
}

export interface EngineProgressEvent {
  depth: number;
  eval_cp: number;
  best_move?: string;
  nodes?: number;
}

// ─── Knowledge Base Types ───

export interface KnowledgeSummaryResponse {
  total_books: number;
  total_chunks: number;
  total_embedded: number;
  books: BookSummary[];
}

export interface BookSummary {
  title: string;
  chunk_count: number;
  chunk_type: string;
  has_embeddings: boolean;
}

export interface IngestionReportResponse {
  books_processed: number;
  chunks_created: number;
  chunks_embedded: number;
  message: string;
}
