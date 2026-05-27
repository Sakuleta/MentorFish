// ─── Opening Explorer View ───
//
// Browse chess openings from the starting position through a move tree.
// Fetches opening data from the Tauri backend via cmd_get_opening.
// Supports repertoire management with add-to-repertoire and chapter panel.

import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { Chess } from "chess.js";
import {
  RotateCcw,
  Undo2,
  ChevronRight,
  Star,
  GripVertical,
  Plus,
  Map,
} from "lucide-react";
import { Board } from "../Board";
import { getTauri } from "../../lib/tauriBridge";
import { useAppStore } from "../../stores";
import type { OpeningNode, OpeningNodeResponse, FEN } from "../../lib/types";
import type { RepertoireChapter, RepertoireEntry } from "../../stores";
import { cn } from "../../lib/utils";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Badge,
  Button,
  Progress,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  Input,
  Label,
} from "../../components/ui";

// ─── Constants ───

const INITIAL_FEN: FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// ─── Helpers ───

/** Apply a UCI move string (e.g. "e2e4") to a FEN and return the resulting FEN. */
function applyUciMove(fen: FEN, uci: string): FEN {
  try {
    const chess = new Chess(fen);
    const result = chess.move({
      from: uci.slice(0, 2),
      to: uci.slice(2, 4),
      promotion: "q",
    });
    if (!result) return fen;
    return chess.fen();
  } catch {
    return fen;
  }
}

/** Build a breadcrumb trail of SAN moves from a history of FENs. */
function buildBreadcrumb(
  fenHistory: FEN[],
  currentNode: OpeningNode | null,
): string[] {
  const crumbs: string[] = [];
  for (let i = 0; i < fenHistory.length; i++) {
    if (i < fenHistory.length - 1) {
      try {
        const chess = new Chess(fenHistory[i]);
        const moves = chess.moves({ verbose: true }) as {
          from: string;
          to: string;
          san: string;
          after: string;
        }[];
        const nextFen = fenHistory[i + 1];
        const move = moves.find((m) => m.after === nextFen);
        if (move) {
          crumbs.push(move.san);
        } else {
          crumbs.push("?");
        }
      } catch {
        crumbs.push("?");
      }
    }
  }
  if (currentNode?.opening_name) {
    // We don't add a separate SAN for the last position; the name covers it
  }
  return crumbs;
}

/** Get turn color from FEN. */
function getTurnColor(fen: FEN): "white" | "black" {
  const parts = fen.split(" ");
  return parts[1] === "w" ? "white" : "black";
}

/** Format win rate as a percentage string. */
function formatWinRate(rate: number): string {
  return `${rate.toFixed(1)}%`;
}

// ─── Inline: CreateRepertoireDialog ───

function CreateRepertoireDialog({
  isOpen,
  onClose,
  onCreated,
}: {
  isOpen: boolean;
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const createRepertoire = useAppStore((s) => s.createRepertoire);
  const [name, setName] = useState("");
  const [color, setColor] = useState<"white" | "black">("white");

  const handleCreate = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    const id = createRepertoire(trimmed, color);
    onCreated(id);
    setName("");
    onClose();
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-88">
        <DialogHeader>
          <DialogTitle>Create Repertoire</DialogTitle>
          <DialogDescription>
            Name your new opening repertoire and choose a color.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4 py-2">
          <div className="flex flex-col gap-1.5">
            <Label>Name</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. My White Repertoire"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
                if (e.key === "Escape") onClose();
              }}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label>Color</Label>
            <div className="flex gap-2">
              <Button
                type="button"
                variant={color === "white" ? "default" : "outline"}
                className="flex-1"
                onClick={() => setColor("white")}
              >
                White
              </Button>
              <Button
                type="button"
                variant={color === "black" ? "default" : "outline"}
                className="flex-1"
                onClick={() => setColor("black")}
              >
                Black
              </Button>
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" onClick={handleCreate} disabled={!name.trim()}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ─── Inline: AddToRepertoireButton ───

function AddToRepertoireButton(props: {
  activeChapterId: string | null;
  isInRepertoire: boolean;
  onAdd: (chapterId: string) => void;
  onRemove: () => void;
  uci?: string;
  san?: string;
  fen?: string;
  openingName?: string;
}) {
  const { activeChapterId, isInRepertoire, onAdd, onRemove } = props;
  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isInRepertoire) {
      onRemove();
    } else if (activeChapterId) {
      onAdd(activeChapterId);
    }
  };

  if (!activeChapterId) return null;

  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn(
        "h-6 w-6 p-0 shrink-0",
        isInRepertoire
          ? "text-warning hover:text-warning"
          : "text-muted-foreground hover:text-warning",
      )}
      onClick={handleClick}
      title={isInRepertoire ? "Remove from repertoire" : "Add to repertoire"}
      aria-label={
        isInRepertoire ? "Remove from repertoire" : "Add to repertoire"
      }
    >
      <Star className={cn("h-4 w-4", isInRepertoire && "fill-warning")} />
    </Button>
  );
}

// ─── Inline: ChapterPanel ───

function ChapterPanel({
  chapters,
  entries,
  activeChapterId,
  setActiveChapterId,
  onAddChapter,
  onReorder,
}: {
  chapters: RepertoireChapter[];
  entries: Record<string, RepertoireEntry[]>;
  activeChapterId: string | null;
  setActiveChapterId: (id: string | null) => void;
  onAddChapter: () => void;
  onReorder: (chapterIds: string[]) => void;
}) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  const sortedChapters = useMemo(() => {
    return [...chapters].sort((a, b) => a.order - b.order);
  }, [chapters]);

  const handleDragStart = (index: number) => {
    setDragIndex(index);
  };

  const handleDragOver = (e: React.DragEvent, index: number) => {
    e.preventDefault();
    if (dragIndex === null || dragIndex === index) return;
    const reordered = [...sortedChapters];
    const [item] = reordered.splice(dragIndex, 1);
    reordered.splice(index, 0, item);
    setDragIndex(index);
    onReorder(reordered.map((c) => c.id));
  };

  const handleDragEnd = () => {
    setDragIndex(null);
  };

  return (
    <Card className="w-60 border-l border-border bg-card flex flex-col shrink-0 rounded-none border-y-0 border-r-0">
      <CardHeader className="px-3 py-3 pb-0">
        <div className="flex items-center justify-between">
          <CardTitle className="text-[13px] font-medium text-primary">
            Chapters
          </CardTitle>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={onAddChapter}
            title="Add chapter"
            aria-label="Add chapter"
          >
            <Plus className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-auto px-0 py-0">
        {sortedChapters.length === 0 && (
          <div className="px-3 py-6 text-center">
            <p className="text-[11px] text-muted-foreground">
              No chapters yet. Click + to add one.
            </p>
          </div>
        )}

        {sortedChapters.map((chapter, index) => {
          const entryCount = (entries[chapter.id] || []).length;
          const isActive = activeChapterId === chapter.id;

          return (
            <div
              key={chapter.id}
              draggable
              onDragStart={() => handleDragStart(index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDragEnd={handleDragEnd}
              onClick={() => setActiveChapterId(isActive ? null : chapter.id)}
              className={cn(
                "flex items-center gap-2 px-3 py-2.5 border-b border-border/50 cursor-pointer transition-colors",
                isActive
                  ? "bg-primary/10 border-l-2 border-l-primary"
                  : "bg-transparent hover:bg-muted/50 border-l-2 border-l-transparent",
              )}
            >
              <GripVertical className="h-3 w-3 text-muted-foreground cursor-grab active:cursor-grabbing select-none shrink-0" />
              <div className="flex-1 min-w-0">
                <span className="text-[12px] text-foreground block truncate">
                  {chapter.name}
                </span>
                <span className="text-[10px] text-muted-foreground">
                  {entryCount} {entryCount === 1 ? "entry" : "entries"}
                </span>
              </div>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}

// ─── Main Component ───

export function ExplorerView() {
  const [fen, setFen] = useState<FEN>(INITIAL_FEN);
  const [history, setHistory] = useState<FEN[]>([]);
  const [node, setNode] = useState<OpeningNode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fetchedFen, setFetchedFen] = useState<FEN | null>(null);
  const fenRef = useRef(fen);

  // ── Repertoire UI state ──
  const [repertoireMode, setRepertoireMode] = useState(false);
  const [activeChapterId, setActiveChapterId] = useState<string | null>(null);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showAddChapterInput, setShowAddChapterInput] = useState(false);
  const [newChapterName, setNewChapterName] = useState("");
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    entryId: string;
  } | null>(null);

  // ── Store state + actions ──
  const {
    repertoires,
    repertoireChapters,
    repertoireEntries,
    activeRepertoireId,
    setActiveRepertoire,
    addChapter,
    addRepertoireEntry,
    removeRepertoireEntry,
    reorderChapters,
  } = useAppStore();

  // Derived: loading is true while data for current FEN has not yet arrived
  const loading = fetchedFen !== fen;

  // Keep fenRef in sync
  useEffect(() => {
    fenRef.current = fen;
  }, [fen]);

  // Close context menu on outside click
  useEffect(() => {
    if (!contextMenu) return;
    const handler = () => setContextMenu(null);
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [contextMenu]);

  // ── Fetch opening data on mount and FEN change ──
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const tauri = await getTauri();
        if (cancelled) return;
        if (!tauri) {
          setNode(null);
          setError("Tauri backend not available");
          setFetchedFen(fen);
          return;
        }
        const response = await tauri.invoke<OpeningNodeResponse>(
          "cmd_get_opening",
          { fen },
        );
        if (cancelled) return;
        setNode(response.node);
        setError(null);
        setFetchedFen(fen);
      } catch (e: unknown) {
        if (cancelled) return;
        console.error("Failed to fetch opening data:", e);
        setError(String(e instanceof Error ? e.message : e));
        setFetchedFen(fen);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [fen]);

  // ── Navigate to a child move ──
  const handleMoveClick = useCallback((uci: string) => {
    const newFen = applyUciMove(fenRef.current, uci);
    setHistory((prev) => [...prev, fenRef.current]);
    setFen(newFen);
  }, []);

  // ── Go back one move ──
  const handleBack = useCallback(() => {
    setHistory((prev) => {
      if (prev.length === 0) return prev;
      const previousFen = prev[prev.length - 1];
      setFen(previousFen);
      return prev.slice(0, -1);
    });
  }, []);

  // ── Reset to initial position ──
  const handleReset = useCallback(() => {
    setFen(INITIAL_FEN);
    setHistory([]);
  }, []);

  // ── Repertoire: get active repertoire ──
  const activeRepertoire = useMemo(
    () => repertoires.find((r) => r.id === activeRepertoireId) ?? null,
    [repertoires, activeRepertoireId],
  );

  // ── Repertoire: get chapters for active repertoire ──
  const activeChapters = useMemo(() => {
    if (!activeRepertoire) return [];
    return activeRepertoire.chapters
      .map((chId) => repertoireChapters[chId])
      .filter(Boolean) as RepertoireChapter[];
  }, [activeRepertoire, repertoireChapters]);

  // ── Repertoire: get entries for active chapter ──
  const activeEntries = useMemo(
    () => (activeChapterId ? (repertoireEntries[activeChapterId] ?? []) : []),
    [activeChapterId, repertoireEntries],
  );

  // ── Repertoire: check if a move is already in the active chapter ──
  const isMoveInRepertoire = useCallback(
    (uci: string, positionFen: string): boolean => {
      return activeEntries.some((e) => e.move === uci && e.fen === positionFen);
    },
    [activeEntries],
  );

  // ── Repertoire: add move to active chapter ──
  const handleAddToRepertoire = useCallback(
    (chapterId: string, moveUci: string, moveSan: string, moveFen: string) => {
      const openingName = node?.opening_name;
      addRepertoireEntry(chapterId, moveFen, moveUci, moveSan, openingName);
    },
    [addRepertoireEntry, node?.opening_name],
  );

  // ── Repertoire: remove move from active chapter ──
  const handleRemoveFromRepertoire = useCallback(
    (moveUci: string, moveFen: string) => {
      const entry = activeEntries.find(
        (e) => e.move === moveUci && e.fen === moveFen,
      );
      if (entry) removeRepertoireEntry(entry.id);
    },
    [activeEntries, removeRepertoireEntry],
  );

  // ── Repertoire: create new chapter ──
  const handleAddChapter = useCallback(() => {
    if (!activeRepertoireId) return;
    setShowAddChapterInput(true);
    setNewChapterName("");
  }, [activeRepertoireId]);

  const handleConfirmChapter = useCallback(() => {
    const trimmed = newChapterName.trim();
    if (!trimmed || !activeRepertoireId) return;
    const chId = addChapter(activeRepertoireId, trimmed);
    setActiveChapterId(chId);
    setShowAddChapterInput(false);
    setNewChapterName("");
  }, [newChapterName, activeRepertoireId, addChapter]);

  // ── Repertoire: reorder chapters ──
  const handleReorderChapters = useCallback(
    (chapterIds: string[]) => {
      if (activeRepertoireId) {
        reorderChapters(activeRepertoireId, chapterIds);
      }
    },
    [activeRepertoireId, reorderChapters],
  );

  // ── Repertoire: select repertoire from dropdown ──
  const handleSelectRepertoire = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const value = e.target.value;
      if (value === "__create_white__") {
        const id = useAppStore
          .getState()
          .createRepertoire("New White Repertoire", "white");
        setActiveRepertoire(id);
        setActiveChapterId(null);
      } else if (value === "__create_black__") {
        const id = useAppStore
          .getState()
          .createRepertoire("New Black Repertoire", "black");
        setActiveRepertoire(id);
        setActiveChapterId(null);
      } else if (value === "") {
        setActiveRepertoire(null);
        setActiveChapterId(null);
      } else {
        setActiveRepertoire(value);
        setActiveChapterId(null);
      }
    },
    [setActiveRepertoire],
  );

  // ── Context menu for notes ──
  const handleContextMenu = useCallback(
    (e: React.MouseEvent, entryId: string) => {
      if (!repertoireMode) return;
      e.preventDefault();
      setContextMenu({ x: e.clientX, y: e.clientY, entryId });
    },
    [repertoireMode],
  );

  const handleAddNote = useCallback(() => {
    if (!contextMenu) return;
    const note = prompt("Enter a note for this repertoire entry:");
    if (note !== null && note.trim()) {
      console.log(`Note for entry ${contextMenu.entryId}: ${note}`);
    }
    setContextMenu(null);
  }, [contextMenu]);

  // ── Derived state ──
  const turnColor = getTurnColor(fen);
  const breadcrumb = buildBreadcrumb(history, node);
  const isAtRoot = history.length === 0;
  const whiteWinRate =
    node?.white_score != null ? node.white_score * 100 : null;
  const blackWinRate =
    node?.white_score != null ? (1 - node.white_score) * 100 : null;

  // ── Render ──
  return (
    <div className="flex h-full relative">
      {/* LEFT: Opening move list */}
      <Card className="w-72 border-r border-border bg-card flex flex-col shrink-0 rounded-none border-y-0 border-l-0">
        {/* Header */}
        <CardHeader className="px-3 py-3 pb-0">
          <div className="flex items-center justify-between">
            <CardTitle className="text-[13px] font-medium text-primary flex items-center gap-1.5">
              <Map className="h-4 w-4" />
              Opening Explorer
            </CardTitle>
            <Button
              variant={repertoireMode ? "default" : "outline"}
              size="sm"
              className="h-7 px-2 text-[10px]"
              onClick={() => {
                setRepertoireMode((prev) => !prev);
                if (!repertoireMode) {
                  if (!activeRepertoireId && repertoires.length > 0) {
                    setActiveRepertoire(repertoires[0].id);
                  }
                }
              }}
            >
              My Repertoire
            </Button>
          </div>
        </CardHeader>

        {/* Repertoire Selector */}
        {repertoireMode && (
          <div className="px-3 py-2 border-b border-border">
            <select
              value={activeRepertoireId ?? ""}
              onChange={handleSelectRepertoire}
              className={cn(
                "flex h-8 w-full rounded-lg border border-border bg-background px-2 text-[11px] text-foreground shadow-sm transition-colors",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring cursor-pointer",
              )}
            >
              <option value="">— No repertoire —</option>
              {repertoires.map((rep) => (
                <option key={rep.id} value={rep.id}>
                  {rep.name} ({rep.color})
                </option>
              ))}
              <option disabled>———</option>
              <option value="__create_white__">+ New White Repertoire</option>
              <option value="__create_black__">+ New Black Repertoire</option>
            </select>
          </div>
        )}

        {/* Active chapter breadcrumb */}
        {repertoireMode && activeChapterId && (
          <div className="px-3 py-1.5 border-b border-border bg-primary/5">
            <span className="text-[11px] text-muted-foreground">
              Chapter:{" "}
              <span className="text-foreground font-medium">
                {repertoireChapters[activeChapterId]?.name ?? "Unknown"}
              </span>
            </span>
            <span className="text-[11px] text-muted-foreground ml-2">
              ({activeEntries.length}{" "}
              {activeEntries.length === 1 ? "entry" : "entries"})
            </span>
          </div>
        )}

        {/* No repertoire / empty state */}
        {repertoireMode && !activeRepertoireId && (
          <div className="px-3 py-8 text-center border-b border-border">
            <p className="text-[11px] text-muted-foreground mb-2">
              No repertoire selected
            </p>
            <p className="text-[10px] text-muted-foreground/60 mb-3">
              Create your first repertoire to start building your opening book.
            </p>
            <Button
              variant="default"
              size="sm"
              className="h-7 text-[11px]"
              onClick={() => setShowCreateDialog(true)}
            >
              Create Repertoire
            </Button>
          </div>
        )}

        {/* Empty chapter state */}
        {repertoireMode &&
          activeRepertoireId &&
          activeChapterId &&
          activeEntries.length === 0 && (
            <div className="px-3 py-4 text-center border-b border-border">
              <p className="text-[11px] text-muted-foreground">
                This chapter has no moves yet. Browse the explorer and click the
                star to add moves.
              </p>
            </div>
          )}

        {/* Breadcrumb trail */}
        {breadcrumb.length > 0 && (
          <div className="px-3 py-2 border-b border-border flex flex-wrap gap-1 items-center text-[11px] text-muted-foreground">
            <button
              onClick={handleReset}
              className="text-primary hover:underline bg-transparent border-0 cursor-pointer p-0"
            >
              start
            </button>
            {breadcrumb.map((san, i) => (
              <span key={i} className="flex items-center gap-0.5">
                <ChevronRight className="h-3 w-3 opacity-40" />
                <span className="text-foreground">{san}</span>
              </span>
            ))}
          </div>
        )}

        {/* Navigation controls */}
        <div className="px-3 py-2 border-b border-border flex gap-2">
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-[12px] px-2"
            onClick={handleBack}
            disabled={isAtRoot}
            icon={<Undo2 className="h-3.5 w-3.5" />}
          >
            Back
          </Button>
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-[12px] px-2"
            onClick={handleReset}
            icon={<RotateCcw className="h-3.5 w-3.5" />}
          >
            Reset
          </Button>
        </div>

        {/* Move list */}
        <div className="flex-1 overflow-auto">
          {loading && (
            <div className="flex items-center justify-center py-6">
              <div className="h-4 w-4 border-2 border-primary border-t-transparent rounded-full animate-spin mr-2" />
              <p className="text-muted-foreground text-center text-xs">
                Loading...
              </p>
            </div>
          )}

          {!loading && error && (
            <p className="text-destructive text-center py-6 text-xs px-3">
              {error}
            </p>
          )}

          {!loading && !error && !node && (
            <p className="text-muted-foreground text-center py-6 text-xs px-3">
              Not in opening database
            </p>
          )}

          {!loading && !error && node && node.children.length === 0 && (
            <p className="text-muted-foreground text-center py-6 text-xs px-3">
              No further moves in database
            </p>
          )}

          {!loading &&
            !error &&
            node?.children.map((move) => {
              const isWhiteTurn = turnColor === "white";
              const winRate =
                node.white_score != null
                  ? isWhiteTurn
                    ? node.white_score * 100
                    : (1 - node.white_score) * 100
                  : null;

              const inRepertoire = repertoireMode
                ? isMoveInRepertoire(move.uci, fen)
                : false;

              const totalGames = node.frequency ?? 1;
              const freqPct = Math.round((move.frequency / totalGames) * 100);

              return (
                <div
                  key={move.uci}
                  onContextMenu={(e) => {
                    if (inRepertoire) {
                      const entry = activeEntries.find(
                        (en) => en.move === move.uci && en.fen === fen,
                      );
                      if (entry) handleContextMenu(e, entry.id);
                    }
                  }}
                >
                  <button
                    onClick={() => handleMoveClick(move.uci)}
                    className="w-full text-left px-3 py-2.5 border-b border-border/50 cursor-pointer bg-transparent hover:bg-primary/5 transition-colors flex items-center gap-2"
                  >
                    {/* Add-to-repertoire star button */}
                    {repertoireMode && (
                      <AddToRepertoireButton
                        uci={move.uci}
                        san={move.san}
                        fen={fen}
                        openingName={node?.opening_name}
                        activeChapterId={activeChapterId}
                        isInRepertoire={inRepertoire}
                        onAdd={(chId) =>
                          handleAddToRepertoire(chId, move.uci, move.san, fen)
                        }
                        onRemove={() =>
                          handleRemoveFromRepertoire(move.uci, fen)
                        }
                      />
                    )}

                    {/* Move SAN */}
                    <span className="text-[13px] font-semibold text-primary min-w-12">
                      {move.san}
                    </span>

                    {/* Win rate */}
                    <Badge
                      variant="outline"
                      className={cn(
                        "text-[10px] min-w-12 justify-center",
                        winRate != null
                          ? winRate >= 50
                            ? "text-success border-success/30"
                            : "text-destructive border-destructive/30"
                          : "text-muted-foreground",
                      )}
                    >
                      {winRate != null ? formatWinRate(winRate) : "—"}
                    </Badge>

                    {/* Frequency */}
                    <div className="flex-1 flex items-center gap-2 justify-end">
                      <Progress
                        value={freqPct}
                        className="h-1 w-16 [&>div]:bg-muted-foreground/40"
                      />
                      <span className="text-[11px] text-muted-foreground tabular-nums">
                        {move.frequency.toLocaleString()}
                      </span>
                    </div>
                  </button>
                </div>
              );
            })}
        </div>
      </Card>

      {/* CENTER: Board */}
      <div className="flex-1 flex flex-col items-center justify-center gap-3 p-4">
        <Board fen={fen} turnColor={turnColor} />

        {/* Opening name */}
        {node?.opening_name && (
          <div className="text-center">
            <p className="text-base font-medium text-foreground m-0">
              {node.opening_name}
            </p>
            {node.eco && (
              <p className="text-[12px] text-muted-foreground mt-0.5 m-0">
                {node.eco}
              </p>
            )}
          </div>
        )}

        {/* Stats footer */}
        {node && node.frequency != null && (
          <div className="flex gap-4 text-[12px] text-muted-foreground">
            <span>{node.frequency.toLocaleString()} games</span>
            {whiteWinRate != null && (
              <>
                <Badge
                  variant="outline"
                  className="text-[10px] text-success border-success/30"
                >
                  W {formatWinRate(whiteWinRate)}
                </Badge>
                <Badge
                  variant="outline"
                  className="text-[10px] text-destructive border-destructive/30"
                >
                  B {formatWinRate(blackWinRate!)}
                </Badge>
              </>
            )}
          </div>
        )}
      </div>

      {/* RIGHT: Chapter Panel (repertoire mode only) */}
      {repertoireMode && activeRepertoire && (
        <ChapterPanel
          chapters={activeChapters}
          entries={repertoireEntries}
          activeChapterId={activeChapterId}
          setActiveChapterId={setActiveChapterId}
          onAddChapter={handleAddChapter}
          onReorder={handleReorderChapters}
        />
      )}

      {/* Inline chapter name input */}
      {showAddChapterInput && (
        <Card className="absolute bottom-4 right-64 z-40 border-border bg-card shadow-lg p-3 flex flex-col gap-2 w-64">
          <Label className="text-[11px] text-muted-foreground">
            Chapter name
          </Label>
          <Input
            value={newChapterName}
            onChange={(e) => setNewChapterName(e.target.value)}
            placeholder="e.g. Sicilian"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === "Enter") handleConfirmChapter();
              if (e.key === "Escape") {
                setShowAddChapterInput(false);
              }
            }}
            className="text-[12px]"
          />
          <div className="flex gap-2 justify-end">
            <Button
              variant="outline"
              size="sm"
              className="h-6 text-[10px] px-2"
              onClick={() => setShowAddChapterInput(false)}
            >
              Cancel
            </Button>
            <Button
              variant="default"
              size="sm"
              className="h-6 text-[10px] px-2"
              onClick={handleConfirmChapter}
              disabled={!newChapterName.trim()}
            >
              Add
            </Button>
          </div>
        </Card>
      )}

      {/* Context menu for notes */}
      {contextMenu && (
        <div
          className="fixed z-50 bg-card border border-border rounded-lg shadow-xl py-1 min-w-32"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            onClick={handleAddNote}
            className="w-full text-left px-3 py-1.5 text-[12px] text-foreground bg-transparent border-0 cursor-pointer hover:bg-primary/10 transition-colors"
          >
            Add note
          </button>
        </div>
      )}

      {/* Create Repertoire Dialog */}
      <CreateRepertoireDialog
        isOpen={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        onCreated={(id) => {
          setActiveRepertoire(id);
          setActiveChapterId(null);
        }}
      />
    </div>
  );
}
