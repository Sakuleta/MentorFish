import type { Dests, Key } from "@lichess-org/chessground/types";
import type { Chessground as ChessgroundFactory } from "@lichess-org/chessground";
import { useEffect, useRef } from "react";

type ChessgroundApi = ReturnType<ChessgroundFactory>;

interface Props {
  fen?: string;
  onMove?: (from: string, to: string) => void;
  lastMove?: [string, string] | null;
  /** Map from origin square to array of destination squares */
  dests?: Map<string, string[]>;
  turnColor?: "white" | "black";
  /** Board orientation: "white" (white at bottom) or "black" (black at bottom) */
  orientation?: "white" | "black";
  /** Whether the current position is check */
  check?: boolean;
  /** Called when the chessground API instance is ready */
  onApiReady?: (api: ChessgroundApi) => void;
  /** Optional CSS class override. Defaults to a viewport-relative square. */
  className?: string;
}

/**
 * Chessground wrapper — matches lichess.org pattern exactly.
 * NEVER passes a plain object for dests; chessground requires Map<Key, Key[]>.
 */
export function Board({
  fen,
  onMove,
  lastMove,
  dests,
  turnColor,
  orientation = "white",
  check,
  onApiReady,
  className,
}: Props) {
  const el = useRef<HTMLDivElement>(null);
  const api = useRef<ChessgroundApi | null>(null);

  // ── Stable prop refs (so the init effect can read latest values) ──
  const fenRef = useRef(fen);
  useEffect(() => { fenRef.current = fen; }, [fen]);

  const lastMoveRef = useRef(lastMove);
  useEffect(() => { lastMoveRef.current = lastMove; }, [lastMove]);

  const destsRef = useRef(dests);
  useEffect(() => { destsRef.current = dests; }, [dests]);

  const turnColorRef = useRef(turnColor);
  useEffect(() => { turnColorRef.current = turnColor; }, [turnColor]);

  const orientationRef = useRef(orientation);
  useEffect(() => { orientationRef.current = orientation; }, [orientation]);

  const checkRef = useRef(check);
  useEffect(() => { checkRef.current = check; }, [check]);

  const onMoveRef = useRef(onMove);
  useEffect(() => { onMoveRef.current = onMove; }, [onMove]);

  const onApiReadyRef = useRef(onApiReady);
  useEffect(() => { onApiReadyRef.current = onApiReady; }, [onApiReady]);

  const apiReadyFired = useRef(false);

  // ── Initialize Chessground ONCE (matching lila's ground.ts pattern) ──
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

      api.current = Chessground(el.current, {
        fen: currentFen,
        orientation: currentOrientation,
        turnColor: currentTurn,
        check: !!checkRef.current,
        coordinates: true,
        animation: { enabled: true, duration: 200 },
        highlight: { lastMove: true, check: true },
        movable: {
          free: false,
          color: currentTurn,
          dests: undefined,
          showDests: true,
          events: {
            after: (orig: Key, dest: Key) => {
              onMoveRef.current?.(orig, dest);
            },
          },
        },
        draggable: {
          enabled: true,
          showGhost: true,
        },
        selectable: {
          enabled: true,
        },
        disableContextMenu: true,
      });

      // Apply any dests that arrived before init completed
      const currentDests = destsRef.current;
      if (currentDests && currentDests.size > 0) {
        api.current.set({
          movable: {
            dests: currentDests as unknown as Dests,
            color: turnColorRef.current || "white",
            free: false,
            showDests: true,
          },
        });
      }

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

  // ── Update board state atomically (matches lila's ground.reload pattern) ──
  useEffect(() => {
    if (!api.current) return;
    const hasDests = dests && dests.size > 0;
    api.current.set({
      fen,
      lastMove: lastMove ? [lastMove[0] as Key, lastMove[1] as Key] : undefined,
      orientation,
      turnColor,
      check: !!check,
      movable: {
        dests: hasDests ? (dests as unknown as Dests) : undefined,
        color: turnColor,
        free: false,
        showDests: true,
      },
    });
  }, [fen, lastMove, dests, turnColor, orientation, check]);

  return (
    <div
      ref={el}
      className={className ?? "w-[min(80vh,60vw)] h-[min(80vh,60vw)]"}
    />
  );
}
