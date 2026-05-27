// ─── Board Heatmap Overlay ───
//
// Draws heatmap circles on the board via chessground's setAutoShapes().
// Supports three heatmap types:
//   - "activity"      → piece mobility / attacked-square counts
//   - "weak_squares"  → isolated pawns, backward pawns, open-file exposure
//   - "king_danger"   → pawn shield gaps + open files near king

import { useEffect, useRef } from "react";
import type { FeatureBundle } from "../../lib/types";

// ─── Chessground shape types (subset) ───
interface CgShape {
  orig: string;
  brush?: string;
}

interface CgApi {
  setAutoShapes: (shapes: CgShape[]) => void;
}

type HeatmapType = "activity" | "weak_squares" | "king_danger";

interface Props {
  api: CgApi | null;
  heatmapType: HeatmapType;
  featureBundle: FeatureBundle | null;
}

// ─── Helpers ───

/**
 * Parse a FEN string to find the rank/file of both kings.
 * Returns { white: file, black: file } where file is 'a'…'h'.
 */
function findKingFiles(fen: string): {
  white: string | null;
  black: string | null;
} {
  const result = { white: null as string | null, black: null as string | null };
  const boardPart = fen.split(" ")[0];
  if (!boardPart) return result;

  const ranks = boardPart.split("/");
  const pieceToColor = (p: string) =>
    p === "K" ? "white" : p === "k" ? "black" : null;
  const files = "abcdefgh";

  for (let rankIdx = 0; rankIdx < ranks.length; rankIdx++) {
    const rank = ranks[rankIdx]!;
    let fileIdx = 0;
    for (let i = 0; i < rank.length; i++) {
      const ch = rank[i]!;
      if (ch >= "1" && ch <= "8") {
        fileIdx += parseInt(ch, 10);
      } else {
        const color = pieceToColor(ch);
        if (color) {
          const file = files[fileIdx];
          if (file) {
            result[color as "white" | "black"] = file;
          }
        }
        fileIdx++;
      }
    }
  }
  return result;
}

/**
 * Given a position's FEN, determine which color's perspective we should use
 * for heatmap rendering (the defending side).
 * Returns "white" or "black" — the side whose weaknesses we highlight.
 */
function sideToMove(fen: string): "white" | "black" {
  const parts = fen.split(" ");
  return parts[1] === "b" ? "black" : "white";
}

// ─── Activity Heatmap ───

function buildActivityShapes(fb: FeatureBundle): CgShape[] {
  const shapes: CgShape[] = [];
  let maxMobility = 1;

  // First pass: find max mobility count
  for (const feat of fb.dynamic) {
    if ("PieceMobility" in feat) {
      const pm = feat.PieceMobility;
      if (pm.legal_move_count > maxMobility) {
        maxMobility = pm.legal_move_count;
      }
    }
  }

  // Second pass: create circles, scaled by mobility
  for (const feat of fb.dynamic) {
    if ("PieceMobility" in feat) {
      const pm = feat.PieceMobility;
      const ratio = Math.min(pm.legal_move_count / Math.max(maxMobility, 1), 1);
      // Brightness: paleGreen (low) → green (medium) → dark green isn't available,
      // so we use paleGreen→green for gradient effect
      const brush =
        ratio > 0.6 ? "green" : ratio > 0.3 ? "paleGreen" : "paleGreen";
      shapes.push({ orig: pm.square, brush });
    }
  }
  return shapes;
}

// ─── Weak Squares Heatmap ───

function buildWeakSquaresShapes(fb: FeatureBundle): CgShape[] {
  const shapes: CgShape[] = [];
  const added = new Set<string>();

  const addShape = (square: string) => {
    if (!added.has(square)) {
      added.add(square);
      shapes.push({ orig: square, brush: "red" });
    }
  };

  // Isolated pawn squares → red
  for (const feat of fb.positional) {
    if ("IsolatedPawn" in feat) {
      addShape(feat.IsolatedPawn.square);
    }
    if ("BackwardPawn" in feat) {
      addShape(feat.BackwardPawn.square);
    }
  }

  // Open / half-open files → highlight squares on those files
  for (const feat of fb.positional) {
    if ("OpenFile" in feat) {
      const file = feat.OpenFile.file;
      // Highlight ranks 3–6 (central zone) on open files
      for (const rank of [3, 4, 5, 6]) {
        addShape(`${file}${rank}`);
      }
    }
    if ("HalfOpenFile" in feat) {
      const { file, color } = feat.HalfOpenFile;
      // Highlight the side's own half of the board
      const defendingRanks = color.toLowerCase() === "white" ? [3, 4] : [5, 6];
      for (const rank of defendingRanks) {
        addShape(`${file}${rank}`);
      }
    }
  }

  return shapes;
}

// ─── King Danger Heatmap ───

function buildKingDangerShapes(fb: FeatureBundle): CgShape[] {
  const shapes: CgShape[] = [];
  const files = "abcdefgh";

  // Locate king positions from FEN
  const kingFiles = findKingFiles(fb.position_fen);
  const stm = sideToMove(fb.position_fen);
  const defendingKingFile = stm === "white" ? kingFiles.white : kingFiles.black;

  // Gather KingSafety features
  let ksFeature: {
    color: string;
    pawn_shield_completeness: number;
    open_files_near_king: number;
  } | null = null;
  for (const feat of fb.positional) {
    if ("KingSafety" in feat) {
      ksFeature = feat.KingSafety;
      break;
    }
  }

  if (!ksFeature || !defendingKingFile) return shapes;

  const kingFileIdx = files.indexOf(defendingKingFile);
  if (kingFileIdx < 0) return shapes;

  // Highlight squares on and around the king's file
  const adjacentFiles = [kingFileIdx];
  if (kingFileIdx > 0) adjacentFiles.push(kingFileIdx - 1);
  if (kingFileIdx < 7) adjacentFiles.push(kingFileIdx + 1);

  const kingRank = defendingKingFile === kingFiles.white ? 1 : 8;
  // Highlight ranks near the king (king rank and ranks above/below)
  const nearRanks =
    stm === "white"
      ? [kingRank, kingRank + 1, kingRank + 2]
      : [kingRank, kingRank - 1, kingRank - 2];

  // Pawn shield gap → more danger circles
  // completeness is 0–1; lower = more danger
  const dangerFactor =
    ksFeature.open_files_near_king * (1 - ksFeature.pawn_shield_completeness);

  for (const fileIdx of adjacentFiles) {
    const file = files[fileIdx];
    if (!file) continue;
    for (const rank of nearRanks) {
      if (rank < 1 || rank > 8) continue;
      const square = `${file}${rank}`;
      // Sparse placement: only place circles every other square for heavy danger
      if (dangerFactor > 1.5) {
        shapes.push({ orig: square, brush: "paleRed" });
      } else if (dangerFactor > 0.5) {
        // Place circles on king's file and adjacent, every other rank
        if (
          (rank % 2 === 0 && fileIdx === kingFileIdx) ||
          (rank % 2 !== 0 && fileIdx !== kingFileIdx)
        ) {
          shapes.push({ orig: square, brush: "paleRed" });
        }
      } else if (dangerFactor > 0) {
        shapes.push({ orig: `${file}${kingRank}`, brush: "paleRed" });
      }
    }
  }

  return shapes;
}

// ─── Component ───

/**
 * BoardHeatmap — draws heatmap circles on the chessground board.
 *
 * Computes shapes from the FeatureBundle and pushes them to chessground
 * whenever api, heatmapType, or featureBundle changes.
 */
export function BoardHeatmap({ api, heatmapType, featureBundle }: Props) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!api || !featureBundle) return;

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      let shapes: CgShape[] = [];

      switch (heatmapType) {
        case "activity":
          shapes = buildActivityShapes(featureBundle);
          break;
        case "weak_squares":
          shapes = buildWeakSquaresShapes(featureBundle);
          break;
        case "king_danger":
          shapes = buildKingDangerShapes(featureBundle);
          break;
      }

      api.setAutoShapes(shapes);
    }, 50);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [api, heatmapType, featureBundle]);

  // Clear shapes on unmount
  useEffect(() => {
    return () => {
      api?.setAutoShapes([]);
    };
  }, [api]);

  return null;
}
