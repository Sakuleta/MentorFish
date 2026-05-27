// ─── Plan Visualizer ───
//
// PRD Section 12.3: Renders multi-move plans from the Pedagogical Agent
// as timed, sequential animated arrow series with synchronized narration.

import { useState, useEffect, useRef, useCallback } from "react";

export interface PlanMove {
  uci: string; // e.g., "e2e4"
  description: string;
}

interface Props {
  api: unknown; // chessground API
  plan: PlanMove[];
  isPlaying: boolean;
  onComplete?: () => void;
}

export function PlanVisualizer({ api, plan, isPlaying, onComplete }: Props) {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [speed, setSpeed] = useState(1);
  const [paused, setPaused] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | undefined>(
    undefined,
  );
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const cg = api as any;

  const clearArrows = useCallback(() => {
    if (cg?.setAutoShapes) {
      cg.setAutoShapes([]);
    }
  }, [cg]);

  const drawMove = useCallback(
    (idx: number) => {
      if (!cg?.setAutoShapes || idx >= plan.length) return;
      const move = plan[idx];
      const orig = move.uci.slice(0, 2);
      const dest = move.uci.slice(2, 4);
      cg.setAutoShapes([
        {
          orig,
          dest,
          brush: "blue",
          label: { text: `${idx + 1}` },
          modifiers: { lineWidth: 6 },
        },
      ]);
    },
    [cg, plan],
  );

  // ── Playback loop ──
  useEffect(() => {
    if (!isPlaying || !cg || plan.length === 0 || paused) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = undefined;
      }
      return;
    }

    const advance = () => {
      setCurrentIndex((prev) => {
        const next = prev + 1;
        if (next >= plan.length) {
          clearArrows();
          onComplete?.();
          return 0;
        }
        drawMove(next);
        return next;
      });
    };

    // Draw first move immediately
    drawMove(currentIndex);

    const delay = 2000 / speed;
    intervalRef.current = setInterval(advance, delay);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [
    isPlaying,
    plan,
    speed,
    paused,
    cg,
    currentIndex,
    drawMove,
    clearArrows,
    onComplete,
  ]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearArrows();
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [clearArrows]);

  const currentMove = plan[currentIndex];

  const togglePause = () => setPaused((p) => !p);
  const goToPrev = () => {
    const prev = Math.max(0, currentIndex - 1);
    setCurrentIndex(prev);
    drawMove(prev);
  };
  const goToNext = () => {
    const next = Math.min(plan.length - 1, currentIndex + 1);
    setCurrentIndex(next);
    drawMove(next);
  };
  const resetPlan = () => {
    setCurrentIndex(0);
    clearArrows();
  };

  if (!isPlaying || plan.length === 0) return null;

  return (
    <>
      {/* Narration overlay — positioned absolutely over the board */}
      {currentMove && (
        <div className="pointer-events-none absolute bottom-4 left-1/2 z-20 -translate-x-1/2 rounded-lg border border-primary/30 bg-surface/90 px-4 py-2 text-sm text-foreground shadow-lg backdrop-blur-sm">
          <span className="font-medium text-primary">
            Move {currentIndex + 1}/{plan.length}:
          </span>{" "}
          {currentMove.description}
        </div>
      )}

      {/* Playback controls */}
      <div className="mt-3 flex items-center justify-center gap-2">
        <button
          onClick={resetPlan}
          className="rounded px-2 py-1 text-xs text-muted transition-colors hover:bg-surface hover:text-foreground cursor-pointer"
          title="Restart"
        >
          ⏮
        </button>
        <button
          onClick={goToPrev}
          className="rounded px-2 py-1 text-xs text-muted transition-colors hover:bg-surface hover:text-foreground cursor-pointer"
          title="Previous"
        >
          ◀
        </button>
        <button
          onClick={togglePause}
          className="rounded bg-primary/10 px-3 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/20 cursor-pointer"
        >
          {paused ? "▶ Play" : "⏸ Pause"}
        </button>
        <button
          onClick={goToNext}
          className="rounded px-2 py-1 text-xs text-muted transition-colors hover:bg-surface hover:text-foreground cursor-pointer"
          title="Next"
        >
          ▶
        </button>

        <span className="mx-2 text-xs text-muted">Speed:</span>
        {[0.5, 1, 2].map((s) => (
          <button
            key={s}
            onClick={() => setSpeed(s)}
            className={`rounded px-2 py-1 text-xs transition-colors cursor-pointer ${
              speed === s
                ? "bg-primary/20 font-medium text-primary"
                : "text-muted hover:bg-surface hover:text-foreground"
            }`}
          >
            {s}x
          </button>
        ))}
      </div>
    </>
  );
}

export default PlanVisualizer;
