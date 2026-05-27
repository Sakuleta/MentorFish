// ─── Board Arrows Overlay ───
//
// Draws SVG arrows on the board via chessground's setAutoShapes().
// Covers: engine best move (blue), MultiPV candidates (gray),
// and the user's last move colored by classification.

import { useEffect, useRef } from "react";
import type { CandidateLine, MoveClassification } from "../../lib/types";

// ─── Chessground shape types (subset) ───
interface CgShape {
  orig: string;
  dest?: string;
  brush?: string;
  label?: { text: string };
}

interface CgApi {
  setAutoShapes: (shapes: CgShape[]) => void;
}

// ─── Classification → brush mapping ───
const CLASSIFICATION_BRUSH: Record<MoveClassification, string> = {
  Best: "green",
  Good: "paleGreen",
  Inaccuracy: "yellow",
  Mistake: "paleRed",
  Blunder: "red",
};

interface Props {
  /** chessground API instance (from Board.onApiReady) */
  api: CgApi | null;
  /** Engine best move in UCI format (e.g. "e2e4") */
  bestMove?: string | null;
  /** MultiPV candidate lines from engine */
  candidates?: CandidateLine[];
  /** User's last played move in UCI format */
  userMove?: string | null;
  /** Classification of the user's move */
  classification?: MoveClassification | null;
}

/**
 * Splits a UCI move like "e2e4" or "e7e8q" into [orig, dest].
 */
function uciToSquares(uci: string): [string, string] {
  return [uci.slice(0, 2), uci.slice(2, 4)];
}

/**
 * BoardArrows — draws arrow overlays on the chessground board.
 *
 * Shapes are computed and flushed to chessground on every render,
 * using a debounced effect to avoid flicker when props change rapidly.
 */
export function BoardArrows({
  api,
  bestMove,
  candidates,
  userMove,
  classification,
}: Props) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!api) return;

    // Debounce to batch rapid prop changes
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      const shapes: CgShape[] = [];

      // ── 1. Engine best move (blue arrow) ──
      if (bestMove) {
        const [orig, dest] = uciToSquares(bestMove);
        shapes.push({ orig, dest, brush: "blue" });
      }

      // ── 2. Alternative candidates (gray arrows, opacity scaled) ──
      if (candidates && candidates.length > 1) {
        // Best eval among candidates (lower cp = better for the side to move)
        // Use absolute cp to handle mate scores gracefully
        const bestEval = candidates[0].eval_cp ?? 0;

        candidates.forEach((cand) => {
          if (cand.pv.length === 0) return;
          const [orig, dest] = uciToSquares(cand.pv[0]);
          // Skip the best move itself (already shown as blue arrow)
          if (bestMove && cand.pv[0] === bestMove) return;

          // Scale opacity: smaller eval loss = darker gray
          const evalLoss = Math.abs((cand.eval_cp ?? 0) - bestEval);
          // Normalised: 0 cp loss = grey, 300+ cp loss = paleGrey
          const brush =
            evalLoss < 100 ? "grey" : evalLoss < 300 ? "paleGrey" : "paleGrey";
          // Optionally annotate with eval
          const labelText =
            cand.eval_cp != null
              ? `${(cand.eval_cp / 100).toFixed(1)}`
              : undefined;

          shapes.push({
            orig,
            dest,
            brush,
            ...(labelText ? { label: { text: labelText } } : {}),
          });
        });
      } else if (candidates && candidates.length === 1) {
        // Single candidate that isn't the best move — show it anyway
        const cand = candidates[0];
        if (cand.pv.length > 0 && cand.pv[0] !== bestMove) {
          const [orig, dest] = uciToSquares(cand.pv[0]);
          shapes.push({ orig, dest, brush: "paleGrey" });
        }
      }

      // ── 3. User's last move (colored by classification) ──
      if (userMove) {
        const [orig, dest] = uciToSquares(userMove);
        // Skip if it's the same as the best move to avoid double-drawing
        if (!bestMove || userMove !== bestMove) {
          const brush = classification
            ? CLASSIFICATION_BRUSH[classification]
            : "green";
          shapes.push({ orig, dest, brush });
        }
        // If userMove == bestMove, recolor the best-move arrow instead
        if (bestMove && userMove === bestMove && classification) {
          const brush = CLASSIFICATION_BRUSH[classification];
          // Replace blue with classified color by filtering + re-adding
          const filtered = shapes.filter(
            (s) => !(s.orig === orig && s.dest === dest && s.brush === "blue"),
          );
          shapes.length = 0;
          shapes.push(...filtered);
          shapes.push({ orig, dest, brush });
        }
      }

      api.setAutoShapes(shapes);
    }, 50);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [api, bestMove, candidates, userMove, classification]);

  // Clear shapes when component unmounts
  useEffect(() => {
    return () => {
      api?.setAutoShapes([]);
    };
  }, [api]);

  // This component renders nothing — it only drives chessground imperatively
  return null;
}
