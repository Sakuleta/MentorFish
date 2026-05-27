import type { Dests } from "@lichess-org/chessground/types";
import { useEffect, useRef } from "react";

interface Props {
  fen?: string;
  onMove?: (from: string, to: string) => void;
  lastMove?: [string, string] | null;
  /** Map from origin square to array of destination squares */
  dests?: Map<string, string[]>;
  turnColor?: "white" | "black";
  /** Board orientation: "white" (white at bottom) or "black" (black at bottom) */
  orientation?: "white" | "black";
  /** Called when the chessground API instance is ready */
  onApiReady?: (api: unknown) => void;
  /** Optional CSS class override. Defaults to a viewport-relative square. */
  className?: string;
}

/**
 * Chessground wrapper — same pattern as lichess.org.
 * NEVER passes a plain object for dests; chessground requires Map<Key, Key[]>.
 */
export function Board({
  fen,
  onMove,
  lastMove,
  dests,
  turnColor,
  orientation = "white",
  onApiReady,
  className,
}: Props) {
  const el = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const api = useRef<any>(null);

  // ── Stable prop refs (so the init effect can read latest values) ──
  const fenRef = useRef(fen);
  useEffect(() => {
    fenRef.current = fen;
  }, [fen]);

  const lastMoveRef = useRef(lastMove);
  useEffect(() => {
    lastMoveRef.current = lastMove;
  }, [lastMove]);

  const destsRef = useRef(dests);
  useEffect(() => {
    destsRef.current = dests;
  }, [dests]);

  const turnColorRef = useRef(turnColor);
  useEffect(() => {
    turnColorRef.current = turnColor;
  }, [turnColor]);

  const orientationRef = useRef(orientation);
  useEffect(() => {
    orientationRef.current = orientation;
  }, [orientation]);

  const onMoveRef = useRef(onMove);
  useEffect(() => {
    onMoveRef.current = onMove;
  }, [onMove]);

  const onApiReadyRef = useRef(onApiReady);
  useEffect(() => {
    onApiReadyRef.current = onApiReady;
  }, [onApiReady]);

  const apiReadyFired = useRef(false);

  // ── Initialize Chessground ONCE ──
  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!el.current) return;
      const { Chessground } = await import("@lichess-org/chessground");
      if (cancelled) return;

      const currentFen =
        fenRef.current ||
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
      const currentTurn = turnColorRef.current || "white";
      const currentOrientation = orientationRef.current || "white";

      console.log("[Board] Initializing chessground with fen:", currentFen);
      api.current = Chessground(el.current, {
        fen: currentFen,
        orientation: currentOrientation,
        turnColor: currentTurn,
        coordinates: true,
        animation: { enabled: true, duration: 200 },
        highlight: { lastMove: true, check: true },
        movable: {
          free: false,
          color: currentTurn,
          dests: undefined,
          showDests: true,
          events: {
            after: (orig: string, dest: string) => {
              console.log("[Board] Move:", orig, "->", dest);
              onMoveRef.current?.(orig, dest);
            },
          },
        },
        draggable: {
          enabled: true,
          showGhost: true,
        },
      });
      console.log("[Board] Chessground initialized, api:", !!api.current);

      // ── CRITICAL: Apply any props that arrived before/during init ──
      const currentDests = destsRef.current;
      const hasDests = currentDests && currentDests.size > 0;
      if (hasDests) {
        console.log(
          "[Board] Applying delayed dests:",
          currentDests!.size,
          "origins",
        );
        api.current.set({
          movable: {
            dests: currentDests as unknown as Dests,
            color: turnColorRef.current || "white",
            free: false,
            showDests: true,
          },
        });
      }

      // Notify parent that the API is ready (only once per mount)
      if (!apiReadyFired.current && api.current) {
        apiReadyFired.current = true;
        onApiReadyRef.current?.(api.current);
      }
    })();
    return () => {
      cancelled = true;
      apiReadyFired.current = false;
      api.current?.destroy();
      api.current = null;
    };
  }, []);

  // ── Update board state atomically after a move ──
  // Chessground's `set()` merges config, but `turnColor` / `movable.color`
  // must be set together with `fen` and `lastMove` in a single call so
  // internal state (highlight, turn indicator, piece interaction) stays
  // consistent.  The Chessground docs show this exact pattern:
  //
  //   board.set({ fen, lastMove, turnColor, movable: { color, dests } })
  //
  // Using separate set() calls leaves `turnColor` stale and can break
  // piece dragging after the opponent moves.
  useEffect(() => {
    if (!api.current) return;
    const hasDests = dests && dests.size > 0;
    console.log(
      "[Board] Updating board:",
      "fen changed,",
      "dests:",
      hasDests ? dests.size + " origins" : "none",
      "turnColor:",
      turnColor,
    );
    api.current.set({
      fen,
      lastMove: lastMove ?? undefined,
      orientation,
      turnColor,
      movable: {
        dests: hasDests ? (dests as unknown as Dests) : undefined,
        color: turnColor,
        free: false,
        showDests: true,
      },
    });
  }, [fen, lastMove, dests, turnColor, orientation]);

  return (
    <div
      ref={el}
      className={className ?? "w-[min(80vh,60vw)] h-[min(80vh,60vw)]"}
    />
  );
}
