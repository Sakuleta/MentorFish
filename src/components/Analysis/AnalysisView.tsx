// ─── Analysis View ───
//
// Full post-game analysis with evaluation graph, move classification,
// coach summary, and streaming deep analysis pipeline.

import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { Chess } from "chess.js";
import { BoardOverlay } from "../Board/BoardOverlay";
import { CoachChat } from "../Chat/CoachChat";
import { useAppStore } from "../../stores";
import type { GameRecord } from "../../stores";
import { getTauri } from "../../lib/tauriBridge";
import type {
  AnalyzePositionResponse,
  StreamingTokenEvent,
  FinalExplanation,
  MoveClassification,
  FEN,
  EngineProgressEvent,
} from "../../lib/types";
import { useResizablePanel } from "../../hooks/useResizablePanel";
import { cn } from "../../lib/utils";
import { Card, CardContent } from "../ui/Card";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "../ui/Tabs";
import { ScrollArea } from "../ui/ScrollArea";
import { Separator } from "../ui/Separator";
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
  TooltipProvider,
} from "../ui/Tooltip";
import {
  BarChart3,
  ChevronLeft,
  ChevronRight,
  AlertCircle,
  Layers,
  Swords,
  Sparkles,
  ArrowLeft,
} from "lucide-react";

// ─── Local Types ───

interface EvalPoint {
  moveNumber: number;
  san: string;
  fen: FEN;
  evalCp: number;
  classification?: MoveClassification;
}

interface ParsedMove extends EvalPoint {
  color: "w" | "b";
}

interface AnalysisState {
  status: "idle" | "loading" | "streaming" | "complete" | "error";
  errorMessage?: string;
  result: AnalyzePositionResponse | null;
}

const STARTING_FEN: FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

const CLASSIFICATION_STYLES: Record<
  MoveClassification,
  { dot: string; bg: string; text: string; label: string }
> = {
  Best: {
    dot: "var(--eval-best)",
    bg: "bg-success/10",
    text: "text-success",
    label: "Best",
  },
  Good: {
    dot: "var(--eval-good)",
    bg: "bg-success/10",
    text: "text-success/80",
    label: "Good",
  },
  Inaccuracy: {
    dot: "var(--eval-inaccuracy)",
    bg: "bg-info/10",
    text: "text-info",
    label: "Inaccuracy",
  },
  Mistake: {
    dot: "var(--eval-mistake)",
    bg: "bg-warning/10",
    text: "text-warning",
    label: "Mistake",
  },
  Blunder: {
    dot: "var(--eval-blunder)",
    bg: "bg-destructive/10",
    text: "text-destructive",
    label: "Blunder",
  },
};

function fenTurnColor(fen: FEN): "white" | "black" {
  return fen.split(" ")[1] === "b" ? "black" : "white";
}

function formatEvalCp(cp: number): string {
  const val = cp / 100;
  if (val === 0) return "0.0";
  return val > 0 ? `+${val.toFixed(1)}` : val.toFixed(1);
}

function applyMoveString(chess: Chess, moveStr: string): boolean {
  try {
    chess.move(moveStr);
    return true;
  } catch {
    try {
      const from = moveStr.slice(0, 2);
      const to = moveStr.slice(2, 4);
      const promotion = moveStr.length > 4 ? moveStr[4] : undefined;
      chess.move({ from, to, promotion });
      return true;
    } catch {
      return false;
    }
  }
}

// ─── EvalGraph Component ───

interface EvalGraphProps {
  points: EvalPoint[];
  currentMoveIndex: number;
  onPointClick: (index: number) => void;
}

function EvalGraph({ points, currentMoveIndex, onPointClick }: EvalGraphProps) {
  const width = 260;
  const height = 140;
  const pad = { top: 16, bottom: 22, left: 36, right: 12 };
  const graphW = width - pad.left - pad.right;
  const graphH = height - pad.top - pad.bottom;

  if (points.length < 2) {
    return (
      <div className="flex items-center justify-center h-[140px] text-muted-foreground text-[11px]">
        Need at least 2 positions for eval graph
      </div>
    );
  }

  const evals = points.map((p) => p.evalCp);
  const minEval = Math.min(...evals, -50);
  const maxEval = Math.max(...evals, 50);
  const absMax = Math.max(Math.abs(minEval), Math.abs(maxEval), 100);
  const yMin = -absMax;
  const yMax = absMax;
  const yRange = yMax - yMin;

  const xScale = (i: number) =>
    pad.left +
    (points.length > 1 ? (i / (points.length - 1)) * graphW : graphW / 2);
  const yScale = (v: number) =>
    pad.top + graphH - ((v - yMin) / yRange) * graphH;

  const pathD = points
    .map((p, i) => {
      const x = xScale(i).toFixed(1);
      const y = yScale(p.evalCp).toFixed(1);
      return `${i === 0 ? "M" : "L"}${x},${y}`;
    })
    .join(" ");

  const tickMagnitude = Math.pow(10, Math.floor(Math.log10(absMax)));
  const yTickStep = Math.max(100, tickMagnitude);
  const yTicks: number[] = [];
  for (let v = yMin; v <= yMax; v += yTickStep) {
    yTicks.push(v);
  }

  const xTickStep = Math.max(1, Math.floor(points.length / 8));

  return (
    <svg
      width={width}
      height={height}
      className="overflow-visible"
      aria-label="Evaluation graph"
    >
      <defs>
        <clipPath id="eval-plot-clip">
          <rect x={pad.left} y={pad.top} width={graphW} height={graphH} />
        </clipPath>
      </defs>

      <rect
        x={pad.left}
        y={pad.top}
        width={graphW}
        height={graphH}
        fill="var(--muted)"
        opacity={0.25}
        rx={4}
      />

      {yTicks.map((v) => (
        <g key={`grid-${v}`}>
          <line
            x1={pad.left}
            y1={yScale(v)}
            x2={pad.left + graphW}
            y2={yScale(v)}
            stroke="var(--border)"
            strokeWidth={0.5}
            strokeOpacity={0.6}
            strokeDasharray={v === 0 ? "none" : "3 3"}
          />
          <text
            x={pad.left - 6}
            y={yScale(v) + 3}
            textAnchor="end"
            fill="var(--muted-foreground)"
            fontSize={9}
            fontFamily="var(--font-mono)"
          >
            {formatEvalCp(v)}
          </text>
        </g>
      ))}

      <line
        x1={pad.left}
        y1={yScale(0)}
        x2={pad.left + graphW}
        y2={yScale(0)}
        stroke="var(--muted-foreground)"
        strokeWidth={1}
        strokeOpacity={0.5}
      />

      <path
        d={pathD}
        fill="none"
        stroke="var(--primary)"
        strokeWidth={1.5}
        strokeLinejoin="round"
        opacity={0.8}
        clipPath="url(#eval-plot-clip)"
      />

      {points.map((p, i) => {
        const style = p.classification
          ? CLASSIFICATION_STYLES[p.classification]
          : null;
        const dotColor = style?.dot ?? "var(--muted-foreground)";
        const isActive = i === currentMoveIndex;
        const cx = xScale(i);
        const cy = yScale(p.evalCp);

        return (
          <g
            key={`dot-${i}`}
            onClick={() => onPointClick(i)}
            style={{ cursor: "pointer" }}
            className="eval-point"
          >
            {isActive && (
              <circle
                cx={cx}
                cy={cy}
                r={7}
                fill="none"
                stroke={dotColor}
                strokeWidth={2}
                opacity={0.35}
              />
            )}
            <circle
              cx={cx}
              cy={cy}
              r={isActive ? 4.5 : 3}
              fill={dotColor}
              stroke={isActive ? "var(--background)" : "none"}
              strokeWidth={isActive ? 1.5 : 0}
            />
            <circle cx={cx} cy={cy} r={9} fill="transparent" />
          </g>
        );
      })}

      {points
        .filter((_, i) => i % xTickStep === 0 || i === points.length - 1)
        .map((p) => {
          const i = points.indexOf(p);
          return (
            <text
              key={`xlabel-${i}`}
              x={xScale(i)}
              y={height - 4}
              textAnchor="middle"
              fill="var(--muted-foreground)"
              fontSize={9}
              fontFamily="var(--font-mono)"
            >
              {p.moveNumber}
            </text>
          );
        })}
    </svg>
  );
}

// ─── Analysis View ───

export function AnalysisView() {
  const currentFen = useAppStore((s) => s.currentFen);
  const setCurrentFen = useAppStore((s) => s.setCurrentFen);
  const persona = useAppStore((s) => s.persona);
  const setActiveView = useAppStore((s) => s.setActiveView);
  const gameResult = useAppStore((s) => s.gameResult);
  const gameHistory = useAppStore((s) => s.gameHistory);
  const streamingTokens = useAppStore((s) => s.streamingTokens);
  const appendStreamToken = useAppStore((s) => s.appendStreamToken);
  const setStreaming = useAppStore((s) => s.setStreaming);
  const resetAnalysis = useAppStore((s) => s.resetAnalysis);

  const [analysisState, setAnalysisState] = useState<AnalysisState>({
    status: "idle",
    result: null,
  });
  const [engineEval, setEngineEval] = useState<number | null>(null);
  const [engineBestMove, setEngineBestMove] = useState<string | null>(null);
  const [engineDepth, setEngineDepth] = useState<number>(0);

  const sortedHistory = useMemo(
    () => [...gameHistory].reverse(),
    [gameHistory],
  );

  const [requestedIndex, setSelectedGameIndex] = useState(0);
  const selectedGameIndex =
    sortedHistory.length === 0
      ? 0
      : Math.min(requestedIndex, sortedHistory.length - 1);

  const initFromHistory = useMemo(() => {
    if (sortedHistory.length === 0) {
      return {
        loaded: null as GameRecord | null,
        moves: [] as ParsedMove[],
        fens: [STARTING_FEN] as FEN[],
        points: [] as EvalPoint[],
        moveIdx: -1,
      };
    }
    const idx = Math.min(selectedGameIndex, sortedHistory.length - 1);
    const game = sortedHistory[idx];
    const chess = new Chess(STARTING_FEN);
    const moves: ParsedMove[] = [];
    const fens: FEN[] = [STARTING_FEN];
    const points: EvalPoint[] = [];
    let failed = false;
    for (let i = 0; i < game.moves.length; i++) {
      const moveStr = game.moves[i];
      if (!applyMoveString(chess, moveStr)) {
        failed = true;
        break;
      }
      const san = chess.history({ verbose: true }).pop()!;
      const fen = chess.fen();
      fens.push(fen);
      const moveNumber = Math.ceil((i + 1) / 2);
      const color = i % 2 === 0 ? "w" : "b";
      moves.push({ moveNumber, san: san.san, fen, evalCp: 0, color });
      points.push({ moveNumber, san: san.san, fen, evalCp: 0 });
    }
    if (failed) {
      return {
        loaded: null as GameRecord | null,
        moves: [] as ParsedMove[],
        fens: [STARTING_FEN] as FEN[],
        points: [] as EvalPoint[],
        moveIdx: -1,
      };
    }
    return { loaded: game, moves, fens, points, moveIdx: fens.length - 1 };
  }, [sortedHistory, selectedGameIndex]);

  const [loadedGame, setLoadedGame] = useState<GameRecord | null>(
    () => initFromHistory.loaded,
  );
  const [parsedMoves, setParsedMoves] = useState<ParsedMove[]>(
    () => initFromHistory.moves,
  );
  const [fenHistory, setFenHistory] = useState<FEN[]>(
    () => initFromHistory.fens,
  );
  const [currentMoveIdx, setCurrentMoveIdx] = useState<number>(
    () => initFromHistory.moveIdx,
  );
  const [evalPoints, setEvalPoints] = useState<EvalPoint[]>(
    () => initFromHistory.points,
  );

  const prevGameIdRef = useRef<string | null>(null);
  useEffect(() => {
    const id = initFromHistory.loaded?.id ?? null;
    if (id !== prevGameIdRef.current) {
      prevGameIdRef.current = id;
      setLoadedGame(initFromHistory.loaded);
      setParsedMoves(initFromHistory.moves);
      setFenHistory(initFromHistory.fens);
      setCurrentMoveIdx(initFromHistory.moveIdx);
      setEvalPoints(initFromHistory.points);
      setAnalysisState({ status: "idle", result: null });
      setEngineEval(null);
      setEngineBestMove(null);
      setEngineDepth(0);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    initFromHistory.loaded?.id,
    initFromHistory.moves,
    initFromHistory.fens,
    initFromHistory.points,
    initFromHistory.moveIdx,
  ]);

  const loadGameByIndex = useCallback((index: number) => {
    setSelectedGameIndex(index);
  }, []);

  const unlistenRef = useRef<(() => void) | null>(null);
  const streamingRef = useRef("");

  useEffect(() => {
    streamingRef.current = streamingTokens;
  }, [streamingTokens]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      const tauri = await getTauri();
      if (!tauri) return;
      const fn = await tauri.listen<EngineProgressEvent>(
        "engine-progress",
        (event) => {
          const { depth, eval_cp, best_move } = event.payload;
          if (depth !== undefined) setEngineDepth(depth);
          if (eval_cp !== undefined) setEngineEval(eval_cp);
          if (best_move) setEngineBestMove(best_move);
        },
      );
      unlisten = fn;
    })();
    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const unlisten = unlistenRef.current;
    return () => {
      unlisten?.();
    };
  }, []);

  const displayFen = useMemo(() => {
    if (currentMoveIdx >= 0 && currentMoveIdx < fenHistory.length) {
      return fenHistory[currentMoveIdx];
    }
    if (fenHistory.length > 0) {
      return fenHistory[fenHistory.length - 1];
    }
    return currentFen;
  }, [currentMoveIdx, fenHistory, currentFen]);

  const turnColor = useMemo(() => fenTurnColor(displayFen), [displayFen]);

  const currentClassification = useMemo<MoveClassification | null>(() => {
    if (currentMoveIdx >= 0 && currentMoveIdx < parsedMoves.length) {
      return parsedMoves[currentMoveIdx].classification ?? null;
    }
    return null;
  }, [currentMoveIdx, parsedMoves]);

  const handleRunAnalysis = useCallback(async () => {
    const targetFen =
      fenHistory.length > 0 ? fenHistory[fenHistory.length - 1] : currentFen;

    setAnalysisState({ status: "loading", result: null });
    resetAnalysis();
    setStreaming(true);

    try {
      const tauri = await getTauri();
      if (!tauri) {
        setAnalysisState({
          status: "error",
          errorMessage:
            "Tauri backend not available. Run with `npm run tauri:dev`.",
          result: null,
        });
        setStreaming(false);
        return;
      }

      const { Channel } = await import("@tauri-apps/api/core");
      const channel = new Channel<StreamingTokenEvent>();
      channel.onmessage = (event: StreamingTokenEvent) => {
        if (event.token) {
          appendStreamToken(event.token);
        }
      };

      setAnalysisState((prev) => ({ ...prev, status: "streaming" }));

      const request = {
        fen: targetFen,
        depth: 22,
        pipeline_type: "PostGame",
        persona,
        ...(loadedGame
          ? { game_id: loadedGame.id, moves: loadedGame.moves }
          : {}),
      };

      const result = await tauri.invoke<AnalyzePositionResponse>(
        "cmd_stream_analyze",
        {
          request,
          onToken: channel,
        },
      );

      const classifiedMoves = classifyMovesFromLayers(
        result.explanation,
        parsedMoves,
      );
      if (classifiedMoves.length > 0) {
        setParsedMoves(classifiedMoves);
      }

      const updatedPoints = buildEvalPoints(
        classifiedMoves.length > 0 ? classifiedMoves : parsedMoves,
        result.engine_eval,
      );
      setEvalPoints(updatedPoints);

      setEngineEval(result.engine_eval);
      setEngineBestMove(result.best_move ?? null);
      setAnalysisState({ status: "complete", result });
      setStreaming(false);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setAnalysisState({
        status: "error",
        errorMessage: msg,
        result: null,
      });
      setStreaming(false);
    }
  }, [
    fenHistory,
    currentFen,
    persona,
    loadedGame,
    parsedMoves,
    resetAnalysis,
    setStreaming,
    appendStreamToken,
  ]);

  const handleMoveClick = useCallback(
    (index: number) => {
      const idx = Math.max(-1, Math.min(index, fenHistory.length - 1));
      setCurrentMoveIdx(idx);
      if (idx >= 0 && idx < fenHistory.length) {
        setCurrentFen(fenHistory[idx]);
      }
    },
    [fenHistory, setCurrentFen],
  );

  const goToMove = useCallback(
    (delta: 1 | -1) => {
      const next = Math.max(
        -1,
        Math.min(currentMoveIdx + delta, fenHistory.length - 1),
      );
      handleMoveClick(next);
    },
    [currentMoveIdx, fenHistory.length, handleMoveClick],
  );

  const handleGameSelect = useCallback(
    (index: number) => {
      loadGameByIndex(index);
    },
    [loadGameByIndex],
  );

  const tacticalItems = useMemo(() => {
    if (!analysisState.result) return null;
    const layers = analysisState.result.explanation.layer_breakdown ?? [];
    const tacticalLayer = layers.find(
      (l) =>
        l.layer_name?.toLowerCase().includes("tactic") ||
        l.layer_name?.toLowerCase().includes("blunder") ||
        l.layer_name?.toLowerCase().includes("mistake"),
    );
    if (!tacticalLayer) return null;
    const lines = tacticalLayer.content
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    return { heading: tacticalLayer.layer_name, lines };
  }, [analysisState.result]);

  const {
    width: leftPanelWidth,
    handleRef: leftPanelHandleRef,
    panelRef: leftPanelPanelRef,
  } = useResizablePanel({
    initialWidth: 240,
    minWidth: 160,
    maxWidth: 400,
    side: "right",
  });
  const {
    width: rightPanelWidth,
    handleRef: rightPanelHandleRef,
    panelRef: rightPanelPanelRef,
  } = useResizablePanel({
    initialWidth: 240,
    minWidth: 160,
    maxWidth: 400,
    side: "left",
  });

  // ── Render ──
  if (gameHistory.length === 0 && parsedMoves.length === 0) {
    return (
      <div className="flex h-full items-center justify-center bg-background">
        <Card className="max-w-sm text-center p-8 shadow-lg">
          <BarChart3 className="w-10 h-10 text-muted-foreground mx-auto mb-4" />
          <h2 className="text-lg font-semibold text-foreground mb-2">
            No Game to Analyze
          </h2>
          <p className="text-sm text-muted-foreground mb-6">
            Play a game first, then review it here.
          </p>
          <Button
            variant="default"
            size="sm"
            onClick={() => setActiveView("board")}
            icon={<Swords className="w-4 h-4" />}
          >
            Go to Play
          </Button>
        </Card>
      </div>
    );
  }

  return (
    <div className="flex h-full bg-background">
      {/* LEFT PANEL */}
      <div
        ref={leftPanelPanelRef}
        style={{ width: leftPanelWidth }}
        className="border-r border-border bg-card/50 flex flex-col shrink-0 relative"
      >
        <div
          ref={leftPanelHandleRef}
          className="absolute top-0 right-0 w-1 h-full cursor-ew-resize hover:bg-primary/30 active:bg-primary/50 transition-colors z-10"
        />

        {/* Header */}
        <div className="px-3 py-3 border-b border-border flex items-center justify-between shrink-0">
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setActiveView("board")}
              title="Back to Play"
            >
              <ArrowLeft className="w-4 h-4" />
            </Button>
            <span className="text-sm font-semibold text-foreground">
              Analysis
            </span>
          </div>
          <Button
            variant="default"
            size="sm"
            onClick={handleRunAnalysis}
            disabled={
              analysisState.status === "loading" ||
              analysisState.status === "streaming"
            }
            loading={
              analysisState.status === "loading" ||
              analysisState.status === "streaming"
            }
            icon={<Sparkles className="w-3.5 h-3.5" />}
          >
            {analysisState.status === "idle" ||
            analysisState.status === "complete"
              ? "Run Deep Analysis"
              : "Analyzing…"}
          </Button>
        </div>

        {/* Game selector */}
        {sortedHistory.length > 1 && (
          <div className="px-3 py-2 border-b border-border shrink-0">
            <select
              value={selectedGameIndex}
              onChange={(e) => handleGameSelect(parseInt(e.target.value))}
              className="w-full text-xs bg-background border border-input rounded-md px-2 py-1.5 text-foreground outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              {sortedHistory.map((g, i) => (
                <option key={g.id} value={i}>
                  {formatGameLabel(g)}
                </option>
              ))}
            </select>
          </div>
        )}
        {sortedHistory.length === 1 && (
          <div className="px-3 py-2 border-b border-border flex items-center shrink-0">
            <span className="text-xs text-muted-foreground">
              1 game displayed
            </span>
          </div>
        )}

        {/* Engine eval bar */}
        {engineEval !== null && (
          <div className="mx-3 mt-2 px-3 py-2 bg-muted/60 rounded-lg text-xs flex items-center gap-2 shrink-0">
            <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
              Eval
            </span>
            <span
              className={cn(
                "font-mono font-semibold",
                engineEval > 0
                  ? "text-success"
                  : engineEval < 0
                    ? "text-destructive"
                    : "text-muted-foreground",
              )}
            >
              {formatEvalCp(engineEval)}
            </span>
            {engineDepth > 0 && (
              <span className="text-muted-foreground text-[10px]">
                depth {engineDepth}
              </span>
            )}
            {engineBestMove && (
              <span className="text-muted-foreground text-[10px] ml-auto font-mono">
                best {engineBestMove}
              </span>
            )}
          </div>
        )}

        {/* Tabs */}
        <Tabs defaultValue="summary" className="flex-1 flex flex-col min-h-0">
          <TabsList className="mx-3 mt-2 shrink-0">
            <TabsTrigger value="summary">Summary</TabsTrigger>
            <TabsTrigger value="tactics">Tactics</TabsTrigger>
            <TabsTrigger value="layers">Layers</TabsTrigger>
          </TabsList>

          <ScrollArea className="flex-1">
            <TabsContent value="summary" className="m-0 p-3 space-y-3">
              {analysisState.status === "loading" && (
                <div className="flex flex-col items-center justify-center py-8 gap-2">
                  <div className="w-5 h-5 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
                  <span className="text-xs text-muted-foreground">
                    Starting analysis…
                  </span>
                </div>
              )}

              {analysisState.status === "streaming" && streamingTokens && (
                <Card className="bg-primary/5 border-primary/10">
                  <CardContent className="p-3">
                    <span className="block text-[10px] text-muted-foreground mb-1 uppercase tracking-wider font-medium">
                      Analysis stream
                    </span>
                    <p className="text-xs text-foreground whitespace-pre-wrap leading-relaxed">
                      {streamingTokens}
                      <span className="inline-block w-1 h-3.5 bg-primary ml-0.5 align-middle animate-blink" />
                    </p>
                  </CardContent>
                </Card>
              )}

              {analysisState.status === "streaming" && !streamingTokens && (
                <div className="flex items-center justify-center py-6 gap-2">
                  <div className="w-4 h-4 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
                  <span className="text-xs text-muted-foreground animate-pulse">
                    Analyzing position…
                  </span>
                </div>
              )}

              {analysisState.status === "error" && (
                <Card className="border-destructive/20 bg-destructive/5">
                  <CardContent className="p-3 space-y-2">
                    <div className="flex items-center gap-1.5 text-destructive">
                      <AlertCircle className="w-3.5 h-3.5" />
                      <span className="text-[10px] uppercase tracking-wider font-semibold">
                        Error
                      </span>
                    </div>
                    <p className="text-xs text-foreground">
                      {analysisState.errorMessage ?? "Analysis failed."}
                    </p>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleRunAnalysis}
                      className="border-destructive/30 text-destructive hover:bg-destructive/10"
                    >
                      Retry
                    </Button>
                  </CardContent>
                </Card>
              )}

              {analysisState.status === "complete" && analysisState.result && (
                <Card className="bg-primary/5 border-primary/10">
                  <CardContent className="p-3 space-y-2">
                    <span className="block text-[10px] text-primary mb-1 uppercase tracking-wider font-semibold">
                      Coach Summary
                    </span>
                    <p className="text-xs text-foreground whitespace-pre-wrap leading-relaxed">
                      {analysisState.result.explanation.text}
                    </p>
                    {analysisState.result.explanation.confidence < 0.8 &&
                      analysisState.result.explanation.low_confidence_note && (
                        <p className="text-[10px] text-warning mt-2 italic">
                          {analysisState.result.explanation.low_confidence_note}
                        </p>
                      )}
                  </CardContent>
                </Card>
              )}

              {analysisState.status === "idle" && (
                <div className="text-center pt-6 text-muted-foreground">
                  <p className="text-xs">
                    Press{" "}
                    <span className="text-primary font-semibold">
                      Run Deep Analysis
                    </span>{" "}
                    to analyze this game.
                  </p>
                  {gameResult && (
                    <p className="text-xs mt-1">
                      Game result:{" "}
                      <span className="font-mono text-foreground">
                        {gameResult}
                      </span>
                    </p>
                  )}
                </div>
              )}

              {parsedMoves.length > 0 && (
                <div className="flex items-center gap-2 text-xs">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => goToMove(-1)}
                    disabled={currentMoveIdx <= 0}
                  >
                    <ChevronLeft className="w-3.5 h-3.5" />
                    Prev
                  </Button>
                  <span className="text-muted-foreground font-mono">
                    {currentMoveIdx >= 0 ? `Move ${currentMoveIdx}` : "Start"} /{" "}
                    {fenHistory.length - 1}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => goToMove(1)}
                    disabled={currentMoveIdx >= fenHistory.length - 1}
                  >
                    Next
                    <ChevronRight className="w-3.5 h-3.5" />
                  </Button>
                </div>
              )}
            </TabsContent>

            <TabsContent value="tactics" className="m-0 p-3 space-y-3">
              {tacticalItems ? (
                <Card className="bg-warning/5 border-warning/10">
                  <CardContent className="p-3 space-y-2">
                    <div className="flex items-center gap-1.5 text-warning">
                      <Swords className="w-3.5 h-3.5" />
                      <span className="text-[10px] uppercase tracking-wider font-semibold">
                        {tacticalItems.heading ?? "Tactical Analysis"}
                      </span>
                    </div>
                    <ul className="space-y-1">
                      {tacticalItems.lines.map((line, i) => (
                        <li
                          key={i}
                          className="text-xs text-foreground flex items-start gap-1.5"
                        >
                          <span className="text-warning mt-0.5">•</span>
                          <span>{line}</span>
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              ) : (
                <p className="text-xs text-muted-foreground text-center py-6">
                  Run analysis to see tactical insights.
                </p>
              )}
            </TabsContent>

            <TabsContent value="layers" className="m-0 p-3 space-y-3">
              {analysisState.status === "complete" &&
              analysisState.result &&
              analysisState.result.explanation.layer_breakdown.length > 0 ? (
                <Card>
                  <CardContent className="p-3 space-y-2">
                    <div className="flex items-center gap-1.5 text-muted-foreground">
                      <Layers className="w-3.5 h-3.5" />
                      <span className="text-[10px] uppercase tracking-wider font-semibold">
                        Layer Breakdown
                      </span>
                    </div>
                    {analysisState.result.explanation.layer_breakdown.map(
                      (layer, i) => (
                        <details key={i} className="group mb-1 last:mb-0">
                          <summary className="text-[11px] text-foreground cursor-pointer hover:text-primary transition-colors list-none flex items-center gap-1">
                            <ChevronRight className="w-3 h-3 text-muted-foreground transition-transform group-open:rotate-90" />
                            <span>
                              {layer.layer_name ?? `Layer ${layer.layer}`}
                            </span>
                            <span className="text-[10px] text-muted-foreground ml-auto font-mono">
                              {Math.round(layer.confidence * 100)}%
                            </span>
                          </summary>
                          <p className="text-[10px] text-muted-foreground mt-1 pl-4 whitespace-pre-wrap leading-relaxed">
                            {layer.content}
                          </p>
                        </details>
                      ),
                    )}
                  </CardContent>
                </Card>
              ) : (
                <p className="text-xs text-muted-foreground text-center py-6">
                  Run analysis to see layer breakdown.
                </p>
              )}
            </TabsContent>
          </ScrollArea>
        </Tabs>
      </div>

      {/* CENTER */}
      <div className="flex-1 flex flex-col items-center gap-3 p-4 overflow-hidden min-w-0">
        <div className="flex-1 flex items-center justify-center min-h-0 w-full overflow-hidden">
          <BoardOverlay
            fen={displayFen}
            bestMove={engineBestMove}
            classification={currentClassification}
            turnColor={turnColor}
            boardClassName="h-full aspect-square max-w-full"
          />
        </div>

        {analysisState.status === "complete" && analysisState.result && (
          <Card className="w-full max-w-md shadow-subtle">
            <CardContent className="p-2.5 flex items-center justify-between text-xs">
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">Eval</span>
                <span
                  className={cn(
                    "font-mono font-semibold",
                    analysisState.result.engine_eval > 0
                      ? "text-success"
                      : analysisState.result.engine_eval < 0
                        ? "text-destructive"
                        : "text-muted-foreground",
                  )}
                >
                  {formatEvalCp(analysisState.result.engine_eval)}
                </span>
              </div>
              {engineDepth > 0 && (
                <span className="text-muted-foreground">
                  Depth {engineDepth}
                </span>
              )}
              {analysisState.result.explanation.confidence > 0 && (
                <span className="text-muted-foreground">
                  Confidence{" "}
                  {Math.round(
                    analysisState.result.explanation.confidence * 100,
                  )}
                  %
                </span>
              )}
            </CardContent>
          </Card>
        )}

        {evalPoints.length >= 2 && (
          <Card className="w-full max-w-md p-3 shadow-subtle">
            <span className="block text-[10px] text-muted-foreground mb-2 px-1 uppercase tracking-wider font-semibold">
              Evaluation Graph
            </span>
            <EvalGraph
              points={evalPoints}
              currentMoveIndex={
                currentMoveIdx >= 0 ? currentMoveIdx : evalPoints.length - 1
              }
              onPointClick={handleMoveClick}
            />
            <div className="flex items-center gap-3 mt-2 px-1 flex-wrap">
              {(
                Object.entries(CLASSIFICATION_STYLES) as [
                  MoveClassification,
                  (typeof CLASSIFICATION_STYLES)[MoveClassification],
                ][]
              ).map(([key, style]) => (
                <span
                  key={key}
                  className="flex items-center gap-1 text-[10px] text-muted-foreground"
                >
                  <span
                    className="inline-block w-2 h-2 rounded-full"
                    style={{ backgroundColor: style.dot }}
                  />
                  {style.label}
                </span>
              ))}
              <span className="flex items-center gap-1 text-[10px] text-muted-foreground">
                <span className="inline-block w-2 h-2 rounded-full bg-muted-foreground" />
                Unknown
              </span>
            </div>
          </Card>
        )}
      </div>

      {/* RIGHT PANEL */}
      <div
        ref={rightPanelPanelRef}
        style={{ width: rightPanelWidth }}
        className="border-l border-border bg-card/50 flex flex-col shrink-0 relative"
      >
        <div
          ref={rightPanelHandleRef}
          className="absolute top-0 left-0 w-1 h-full cursor-ew-resize hover:bg-primary/30 active:bg-primary/50 transition-colors z-10"
        />

        {/* Classification strip */}
        {parsedMoves.length > 0 && (
          <div className="px-3 pt-3 pb-1 shrink-0">
            <TooltipProvider>
              <div className="flex items-center gap-1.5 flex-wrap">
                {parsedMoves.map((m, i) => (
                  <Tooltip key={i}>
                    <TooltipTrigger asChild>
                      <button
                        onClick={() =>
                          handleMoveClick(fenHistory.indexOf(m.fen))
                        }
                        className={cn(
                          "w-2 h-2 rounded-full transition-transform hover:scale-125",
                          !m.classification && "bg-muted-foreground",
                        )}
                        style={
                          m.classification
                            ? {
                                backgroundColor:
                                  CLASSIFICATION_STYLES[m.classification].dot,
                              }
                            : undefined
                        }
                      />
                    </TooltipTrigger>
                    <TooltipContent side="top" className="text-[10px]">
                      {m.moveNumber}. {m.san}
                      {m.classification ? ` — ${m.classification}` : ""}
                    </TooltipContent>
                  </Tooltip>
                ))}
              </div>
            </TooltipProvider>
            <Separator className="mt-2" />
          </div>
        )}

        {/* Move List */}
        <div className="flex flex-col shrink-0 max-h-[45%] min-h-[120px]">
          <div className="px-3 py-2 border-b border-border flex items-center justify-between shrink-0">
            <span className="text-sm font-semibold text-foreground">
              Move List
            </span>
            {loadedGame && loadedGame.result !== "*" && (
              <Badge variant="outline" className="font-mono text-[10px]">
                {loadedGame.result}
              </Badge>
            )}
          </div>
          <ScrollArea className="flex-1 p-2">
            {parsedMoves.length === 0 ? (
              <p className="text-muted-foreground text-center pt-4 text-xs">
                {loadedGame
                  ? "No moves recorded."
                  : "No moves to display. Play a game first."}
              </p>
            ) : (
              <table className="w-full">
                <tbody>
                  {(() => {
                    const rows: Array<{
                      num: number;
                      white?: ParsedMove;
                      black?: ParsedMove;
                    }> = [];
                    for (let i = 0; i < parsedMoves.length; i += 2) {
                      const num = Math.ceil((i + 1) / 2);
                      rows.push({
                        num,
                        white: parsedMoves[i],
                        black: parsedMoves[i + 1],
                      });
                    }
                    return rows.map((row) => (
                      <tr
                        key={row.num}
                        className="hover:bg-muted/40 transition-colors"
                      >
                        <td className="text-muted-foreground text-[10px] w-6 pr-1 text-right align-middle font-mono">
                          {row.num}.
                        </td>
                        {row.white && (
                          <td
                            onClick={() =>
                              handleMoveClick(
                                fenHistory.indexOf(row.white!.fen),
                              )
                            }
                            className={cn(
                              "px-1.5 py-0.5 rounded cursor-pointer align-middle",
                              currentMoveIdx >= 0 &&
                                fenHistory[currentMoveIdx] === row.white.fen
                                ? "bg-primary/15 text-primary"
                                : row.white.classification
                                  ? CLASSIFICATION_STYLES[
                                      row.white.classification
                                    ].bg
                                  : "",
                              "hover:bg-primary/10 transition-colors",
                            )}
                          >
                            <div className="flex items-center justify-between gap-1">
                              <span
                                className={cn(
                                  "text-xs font-mono",
                                  currentMoveIdx >= 0 &&
                                    fenHistory[currentMoveIdx] === row.white.fen
                                    ? "font-medium text-primary"
                                    : "text-foreground",
                                )}
                              >
                                {row.white.san}
                              </span>
                              {row.white.classification && (
                                <Badge
                                  variant="outline"
                                  className={cn(
                                    "text-[9px] px-1 py-0 border-0",
                                    CLASSIFICATION_STYLES[
                                      row.white.classification
                                    ].text,
                                  )}
                                >
                                  {
                                    CLASSIFICATION_STYLES[
                                      row.white.classification
                                    ].label
                                  }
                                </Badge>
                              )}
                            </div>
                          </td>
                        )}
                        {!row.white && <td className="px-1.5 py-0.5" />}
                        {row.black && (
                          <td
                            onClick={() =>
                              handleMoveClick(
                                fenHistory.indexOf(row.black!.fen),
                              )
                            }
                            className={cn(
                              "px-1.5 py-0.5 rounded cursor-pointer align-middle",
                              currentMoveIdx >= 0 &&
                                fenHistory[currentMoveIdx] === row.black.fen
                                ? "bg-primary/15 text-primary"
                                : row.black.classification
                                  ? CLASSIFICATION_STYLES[
                                      row.black.classification
                                    ].bg
                                  : "",
                              "hover:bg-primary/10 transition-colors",
                            )}
                          >
                            <div className="flex items-center justify-between gap-1">
                              <span
                                className={cn(
                                  "text-xs font-mono",
                                  currentMoveIdx >= 0 &&
                                    fenHistory[currentMoveIdx] === row.black.fen
                                    ? "font-medium text-primary"
                                    : "text-foreground",
                                )}
                              >
                                {row.black.san}
                              </span>
                              {row.black.classification && (
                                <Badge
                                  variant="outline"
                                  className={cn(
                                    "text-[9px] px-1 py-0 border-0",
                                    CLASSIFICATION_STYLES[
                                      row.black.classification
                                    ].text,
                                  )}
                                >
                                  {
                                    CLASSIFICATION_STYLES[
                                      row.black.classification
                                    ].label
                                  }
                                </Badge>
                              )}
                            </div>
                          </td>
                        )}
                        {!row.black && <td className="px-1.5 py-0.5" />}
                      </tr>
                    ));
                  })()}
                </tbody>
              </table>
            )}
          </ScrollArea>
        </div>

        {/* Coach Chat */}
        <div className="flex-1 flex flex-col min-h-0">
          <div className="px-3 py-2 border-b border-border shrink-0">
            <span className="text-sm font-semibold text-foreground">
              Coach Chat
            </span>
          </div>
          <CoachChat fen={displayFen} />
        </div>
      </div>
    </div>
  );
}

// ─── Analysis Helpers ───

function classifyMovesFromLayers(
  explanation: FinalExplanation,
  moves: ParsedMove[],
): ParsedMove[] {
  if (moves.length === 0) return moves;
  const classified = moves.map((m) => ({ ...m }));
  const allText = explanation.layer_breakdown
    .map((l) => l.content)
    .join("\n")
    .toLowerCase();
  const keywords: Record<MoveClassification, string[]> = {
    Blunder: ["blunder", "??", "terrible", "losing"],
    Mistake: ["mistake", "?", "error", "bad move"],
    Inaccuracy: ["inaccuracy", "?!", "imprecise", "not best"],
    Good: ["good", "nice", "strong", "well played"],
    Best: ["best", "!!", "excellent", "brilliant"],
  };
  for (let i = 0; i < classified.length; i++) {
    const move = classified[i];
    if (move.classification) continue;
    const sanLower = move.san.toLowerCase();
    if (!allText.includes(sanLower)) continue;
    const idx = allText.indexOf(sanLower);
    const contextStart = Math.max(0, idx - 60);
    const contextEnd = Math.min(allText.length, idx + sanLower.length + 60);
    const context = allText.slice(contextStart, contextEnd);
    for (const [classification, terms] of Object.entries(keywords)) {
      const matched = terms.some((term) => context.includes(term));
      if (matched) {
        move.classification = classification as MoveClassification;
        break;
      }
    }
  }
  return classified;
}

function buildEvalPoints(
  moves: ParsedMove[],
  finalEvalCp: number,
): EvalPoint[] {
  if (moves.length === 0) return [];
  const points: EvalPoint[] = [];
  for (let i = 0; i < moves.length; i++) {
    points.push({
      moveNumber: moves[i].moveNumber,
      san: moves[i].san,
      fen: moves[i].fen,
      evalCp: moves[i].evalCp ?? 0,
      classification: moves[i].classification,
    });
  }
  if (moves.length > 0) {
    const last = moves[moves.length - 1];
    points.push({
      moveNumber: last.moveNumber,
      san: "★",
      fen: last.fen,
      evalCp: finalEvalCp,
    });
  }
  return points;
}

function formatGameLabel(game: GameRecord): string {
  const date = new Date(game.playedAt).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
  return `${date} · ${game.result}${game.opening ? ` · ${game.opening}` : ""}`;
}
