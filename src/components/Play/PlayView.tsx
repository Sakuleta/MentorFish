// ─── Play View ───
// Premium play interface with board, move list, captured pieces,
// game controls, and integrated coach chat.

import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import { Chess } from "chess.js";
import { Board } from "../Board/Board";
import { CoachChat } from "../Chat/CoachChat";
import { useAppStore } from "../../stores";
import { makeMove, aiMove } from "../../lib/tauriBridge";
import { cn } from "../../lib/utils";
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
  CardFooter,
} from "../ui/Card";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { ScrollArea } from "../ui/ScrollArea";
import { Progress } from "../ui/Progress";
import { Separator } from "../ui/Separator";
import {
  RotateCcw,
  Flag,
  Handshake,
  FlipHorizontal,
  User,
  Bot,
  ChevronLeft,
  Activity,
  Swords,
} from "lucide-react";

const STARTING_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/** Compute legal destinations as a Map for chessground. */
function computeDests(chess: Chess): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const m of chess.moves({ verbose: true })) {
    const list = map.get(m.from) || [];
    list.push(m.to);
    map.set(m.from, list);
  }
  return map;
}

export function PlayView() {
  // ── Store ──
  const boardOrientation = useAppStore((s) => s.boardOrientation);
  const setBoardOrientation = useAppStore((s) => s.setBoardOrientation);
  const engineProgress = useAppStore((s) => s.engineProgress);
  const playMode = useAppStore((s) => s.playMode);
  const playStrength = useAppStore((s) => s.playStrength);
  const addGameToHistory = useAppStore((s) => s.addGameToHistory);
  const setGameResult = useAppStore((s) => s.setGameResult);
  const setCurrentFen = useAppStore((s) => s.setCurrentFen);

  // ── Local game state ──
  const chessRef = useRef(new Chess());
  const [fen, setFen] = useState(STARTING_FEN);
  const [moves, setMoves] = useState<string[]>([]);
  const [gameOver, setGameOver] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [check, setCheck] = useState(false);
  const [captured, setCaptured] = useState<{ w: string[]; b: string[] }>({
    w: [],
    b: [],
  });
  const [lastMove, setLastMove] = useState<[string, string] | null>(null);
  const [isThinking, setIsThinking] = useState(false);
  const [engineError, setEngineError] = useState(false);

  const turnColor = useMemo(
    () => (fen.split(" ")[1] === "b" ? "black" : "white"),
    [fen],
  );

  const isUserTurn = turnColor === boardOrientation;

  const dests = useMemo(() => {
    if (gameOver || isThinking) return new Map<string, string[]>();
    if (playMode !== "human" && !isUserTurn) return new Map<string, string[]>();
    const g = new Chess(fen);
    return computeDests(g);
  }, [fen, gameOver, isThinking, playMode, isUserTurn]);

  const moveRows = useMemo(() => {
    const rows: { num: number; white?: string; black?: string }[] = [];
    for (let i = 0; i < moves.length; i += 2) {
      rows.push({
        num: Math.floor(i / 2) + 1,
        white: moves[i],
        black: moves[i + 1],
      });
    }
    return rows;
  }, [moves]);

  const statusText = useMemo(() => {
    if (gameOver) {
      if (result === "1-0") return "White wins";
      if (result === "0-1") return "Black wins";
      if (result === "1/2-1/2") return "Draw";
      return "Game over";
    }
    if (isThinking || !isUserTurn) return "Opponent thinking…";
    return "Your move";
  }, [gameOver, result, isThinking, isUserTurn]);

  const moveNumber = useMemo(() => {
    const num = fen.split(" ")[5];
    return num ?? "1";
  }, [fen]);

  // ── Sync all derived state from the chess.js instance ──
  const syncFromChess = useCallback(
    (g: Chess) => {
      const newFen = g.fen();
      setFen(newFen);
      setCurrentFen(newFen);
      setMoves(g.history());
      setCheck(g.isCheck());
      setEngineError(false);

      const historyVerbose = g.history({ verbose: true });
      if (historyVerbose.length > 0) {
        const last = historyVerbose[historyVerbose.length - 1];
        setLastMove([last.from, last.to]);
      } else {
        setLastMove(null);
      }

      const nextCaptured: { w: string[]; b: string[] } = { w: [], b: [] };
      const tmp = new Chess();
      for (const san of g.history()) {
        const m = tmp.move(san);
        if (m?.captured) {
          const piece = m.captured.toUpperCase();
          const victim = m.color === "w" ? "b" : "w";
          nextCaptured[victim].push(piece);
        }
      }
      setCaptured(nextCaptured);

      if (g.isGameOver()) {
        setGameOver(true);
        let res = "*";
        if (g.isCheckmate()) res = g.turn() === "w" ? "0-1" : "1-0";
        else if (g.isDraw()) res = "1/2-1/2";
        setResult(res);
        setGameResult(res);
        addGameToHistory({
          id: `game-${Date.now()}`,
          fen: newFen,
          moves: g.history(),
          result: res,
          playedAt: new Date().toISOString(),
          playerColor: boardOrientation,
        });
      }
    },
    [setCurrentFen, setGameResult, addGameToHistory, boardOrientation],
  );

  // ── Actions (matches lila's server-authoritative move flow) ──
  const handleMove = useCallback(
    async (from: string, to: string) => {
      if (gameOver || isThinking) return;
      if (playMode !== "human" && !isUserTurn) return;

      const g = chessRef.current;
      const fenBefore = g.fen();
      const move = g.move({ from, to, promotion: "q" });
      if (!move) return;

      let uci = from + to;
      if (move.promotion) uci += move.promotion;

      syncFromChess(g);

      if (playMode === "human") return;

      setIsThinking(true);
      try {
        const res = await makeMove({
          fen: fenBefore,
          uci,
          vsAi: true,
          strengthMode: playMode === "training" ? "training" : "full",
          targetElo: playStrength,
        });

        // Use backend FEN as source of truth to avoid chess.js / shakmaty desync
        const authoritativeFen = res.aiFen ?? res.fen;
        if (authoritativeFen) {
          g.load(authoritativeFen);
        }

        syncFromChess(g);
      } catch (err) {
        console.error("Engine move failed:", err);
        // Keep the user's move on the board — don't roll back
        // (like lila: piece stays, game continues, user can undo manually)
        setEngineError(true);
      } finally {
        setIsThinking(false);
      }
    },
    [gameOver, isThinking, playMode, isUserTurn, playStrength, syncFromChess],
  );

  const handleNewGame = useCallback(async () => {
    const g = new Chess();
    chessRef.current = g;
    setFen(STARTING_FEN);
    setCurrentFen(STARTING_FEN);
    setMoves([]);
    setGameOver(false);
    setResult(null);
    setGameResult(null);
    setCheck(false);
    setEngineError(false);
    setCaptured({ w: [], b: [] });
    setLastMove(null);
    setIsThinking(false);

    // If playing as Black vs AI, trigger Stockfish's opening move
    if (playMode !== "human" && boardOrientation === "black") {
      setIsThinking(true);
      try {
        const res = await aiMove(
          STARTING_FEN,
          playMode === "training" ? "training" : "full",
          playStrength,
        );
        if (res.aiFen) {
          g.load(res.aiFen);
        } else if (res.aiMove) {
          g.move({ from: res.aiMove.slice(0, 2), to: res.aiMove.slice(2, 4), promotion: res.aiMove[4] || undefined });
        }
        syncFromChess(g);
      } catch (err) {
        console.error("AI opening move failed:", err);
      } finally {
        setIsThinking(false);
      }
    }
  }, [
    playMode,
    boardOrientation,
    playStrength,
    setCurrentFen,
    setGameResult,
    syncFromChess,
  ]);

  // Auto-trigger AI opening move on mount when user plays Black
  useEffect(() => {
    if (playMode === "human") return;
    if (boardOrientation === "white") return;
    if (moves.length > 0) return;
    if (gameOver) return;

    const g = chessRef.current;
    let cancelled = false;

    const run = async () => {
      try {
        setIsThinking(true);
        const res = await aiMove(
          STARTING_FEN,
          playMode === "training" ? "training" : "full",
          playStrength,
        );
        if (cancelled) return;
        if (res.aiFen) {
          g.load(res.aiFen);
        } else if (res.aiMove) {
          g.move({
            from: res.aiMove.slice(0, 2),
            to: res.aiMove.slice(2, 4),
            promotion: res.aiMove[4] || undefined,
          });
        }
        syncFromChess(g);
      } catch (err) {
        console.error("AI opening move failed:", err);
      } finally {
        if (!cancelled) setIsThinking(false);
      }
    };

    run();
    return () => {
      cancelled = true;
    };
  }, [
    playMode,
    boardOrientation,
    moves.length,
    gameOver,
    playStrength,
    syncFromChess,
  ]);

  const handleResign = useCallback(() => {
    if (gameOver) return;
    setGameOver(true);
    setEngineError(false);
    const res = boardOrientation === "white" ? "0-1" : "1-0";
    setResult(res);
    setGameResult(res);
    addGameToHistory({
      id: `game-${Date.now()}`,
      fen,
      moves,
      result: res,
      playedAt: new Date().toISOString(),
      playerColor: boardOrientation,
    });
  }, [boardOrientation, gameOver, setGameResult, addGameToHistory, fen, moves]);

  const handleDraw = useCallback(() => {
    if (gameOver) return;
    setGameOver(true);
    setEngineError(false);
    setResult("1/2-1/2");
    setGameResult("1/2-1/2");
    addGameToHistory({
      id: `game-${Date.now()}`,
      fen,
      moves,
      result: "1/2-1/2",
      playedAt: new Date().toISOString(),
      playerColor: boardOrientation,
    });
  }, [gameOver, setGameResult, addGameToHistory, boardOrientation, fen, moves]);

  const handleFlip = useCallback(() => {
    setBoardOrientation(boardOrientation === "white" ? "black" : "white");
  }, [boardOrientation, setBoardOrientation]);

  return (
    <div className="flex h-full gap-4 p-4 bg-background overflow-hidden">
      {/* ═══ LEFT PANEL ═══ */}
      <div className="w-64 shrink-0 hidden lg:flex flex-col gap-3">
        {/* Game Info */}
        <Card className="shadow-subtle">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Game Info</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <User className="w-4 h-4 text-primary" />
                <span className="text-sm font-medium">You</span>
              </div>
              <span className="text-xs text-muted-foreground">
                {boardOrientation === "white" ? "White" : "Black"}
              </span>
            </div>
            <Separator />
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Bot className="w-4 h-4 text-muted-foreground" />
                <span className="text-sm font-medium">
                  {playMode === "human" ? "Human" : "Stockfish"}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {playMode === "human" ? "?" : `${playStrength} Elo`}
              </span>
            </div>

            {/* Captured pieces */}
            <div className="pt-1">
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
                Captured
              </span>
              <div className="flex flex-wrap gap-1 mt-1.5">
                {captured.w.length === 0 && captured.b.length === 0 && (
                  <span className="text-xs text-muted-foreground italic">
                    None yet
                  </span>
                )}
                {captured.w.map((p, i) => (
                  <span
                    key={`w-${i}`}
                    className="inline-flex items-center justify-center w-5 h-5 rounded bg-muted text-[10px] font-mono font-medium text-foreground"
                  >
                    {p}
                  </span>
                ))}
                {captured.b.map((p, i) => (
                  <span
                    key={`b-${i}`}
                    className="inline-flex items-center justify-center w-5 h-5 rounded bg-muted text-[10px] font-mono font-medium text-muted-foreground"
                  >
                    {p}
                  </span>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Move List */}
        <Card className="flex-1 min-h-0 flex flex-col shadow-subtle">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Moves</CardTitle>
          </CardHeader>
          <CardContent className="flex-1 min-h-0 p-0">
            <ScrollArea className="h-full px-4 pb-4">
              <div className="space-y-0">
                {moveRows.map((row, i) => (
                  <div
                    key={row.num}
                    className={cn(
                      "flex items-center px-2 py-1 rounded text-sm",
                      i % 2 === 0 ? "bg-muted/40" : "bg-transparent",
                    )}
                  >
                    <span className="w-8 text-xs text-muted-foreground font-mono text-right pr-2 shrink-0">
                      {row.num}.
                    </span>
                    <span className="flex-1 font-mono text-foreground">
                      {row.white ?? ""}
                    </span>
                    <span className="flex-1 font-mono text-foreground">
                      {row.black ?? ""}
                    </span>
                  </div>
                ))}
                {moves.length === 0 && (
                  <p className="text-xs text-muted-foreground text-center py-6">
                    No moves yet.
                  </p>
                )}
              </div>
            </ScrollArea>
          </CardContent>
          <CardFooter className="gap-2 pt-0">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                if (moves.length === 0) return;
                chessRef.current.undo();
                syncFromChess(chessRef.current);
              }}
              disabled={moves.length === 0 || isThinking}
              className="flex-1"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
              Undo
            </Button>
          </CardFooter>
        </Card>

        {/* Controls */}
        <Card className="shadow-subtle">
          <CardContent className="p-3 flex flex-wrap gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={handleNewGame}
              icon={<RotateCcw className="w-3.5 h-3.5" />}
            >
              New Game
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleResign}
              icon={<Flag className="w-3.5 h-3.5" />}
            >
              Resign
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleDraw}
              icon={<Handshake className="w-3.5 h-3.5" />}
            >
              Draw
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleFlip}
              icon={<FlipHorizontal className="w-3.5 h-3.5" />}
            >
              Flip
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* ═══ CENTER ═══ */}
      <div className="flex-1 flex flex-col items-center justify-center min-w-0 gap-3 overflow-hidden">
        {/* Top bar */}
        <Card className="w-full max-w-xl shadow-subtle">
          <CardContent className="p-2.5 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Swords className="w-4 h-4 text-muted-foreground" />
              <span className="text-sm font-medium">Move {moveNumber}</span>
              <Separator orientation="vertical" className="h-4" />
              <span className="text-xs text-muted-foreground">
                {turnColor === "white" ? "White to move" : "Black to move"}
              </span>
            </div>

            <div className="flex items-center gap-3">
              {engineProgress && (
                <>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
                      Depth {engineProgress.depth}
                    </span>
                    <Progress
                      value={Math.min(100, (engineProgress.depth / 30) * 100)}
                      className="w-20 h-1.5"
                    />
                  </div>
                  <Badge variant="outline" className="font-mono text-xs">
                    {engineProgress.evalCp > 0 ? "+" : ""}
                    {(engineProgress.evalCp / 100).toFixed(1)}
                  </Badge>
                </>
              )}
              <Badge
                variant={gameOver ? "destructive" : "default"}
                className="text-xs"
              >
                {statusText}
              </Badge>
            </div>
          </CardContent>
        </Card>

        {/* Board */}
        <div className="relative flex-1 flex items-center justify-center min-h-0 w-full overflow-hidden">
          <Board
            fen={fen}
            onMove={handleMove}
            lastMove={lastMove}
            dests={dests}
            turnColor={turnColor}
            orientation={boardOrientation}
            check={check}
            className="h-full aspect-square max-w-full"
          />
          {engineError && (
            <div className="absolute top-2 left-1/2 -translate-x-1/2 z-10">
              <Badge variant="destructive" className="text-xs whitespace-nowrap">
                Engine error — use undo or New Game
              </Badge>
            </div>
          )}
        </div>
      </div>

      {/* ═══ RIGHT PANEL: Coach Chat ═══ */}
      <div className="w-72 shrink-0 hidden xl:flex flex-col">
        <Card className="flex-1 flex flex-col h-full shadow-subtle">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm flex items-center gap-2">
              <Activity className="w-4 h-4 text-primary" />
              Coach
            </CardTitle>
          </CardHeader>
          <CardContent className="flex-1 min-h-0 p-0 overflow-hidden">
            <CoachChat fen={fen} />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
