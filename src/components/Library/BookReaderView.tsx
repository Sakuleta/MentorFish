// ─── Book Reader View ───
//
// Per PRD Section 3.7: page-by-page reading, progress tracking,
// "Explain This", "Quiz Me", bookmark support, and session tracking.

import { useState, useEffect, useCallback, useRef } from "react";
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  Lightbulb,
  BrainCircuit,
  Bookmark,
  X,
  Check,
  BookOpen,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "../../lib/utils";
import { Button, Card, Progress, Badge, ScrollArea } from "../../components/ui";

// ─── Types ───

interface BookChunk {
  id: string;
  chunk_type: string;
  content: string;
  source: string;
  position_fen?: string;
  opening_eco?: string;
  similarity: number;
}

interface GetBookChunksResponse {
  chunks: BookChunk[];
}

// ─── localStorage Keys ───

function progressKey(bookTitle: string) {
  return `book-progress-${bookTitle}`;
}

function bookmarkKey(bookTitle: string) {
  return `book-bookmarks-${bookTitle}`;
}

function loadSavedPage(bookTitle: string): number {
  try {
    const saved = localStorage.getItem(progressKey(bookTitle));
    if (saved) {
      const page = parseInt(saved, 10);
      if (!isNaN(page) && page >= 0) return page;
    }
  } catch {
    // localStorage may not be available
  }
  return 0;
}

function loadSavedBookmarks(
  bookTitle: string,
): { page: number; note: string; createdAt: string }[] {
  try {
    const saved = localStorage.getItem(bookmarkKey(bookTitle));
    if (saved) return JSON.parse(saved);
  } catch {
    // ignore
  }
  return [];
}

// ─── Component ───

interface BookReaderViewProps {
  bookTitle: string;
  onBack: () => void;
  /** Optional callback for "Explain This" — sends content to analysis */
  onExplain?: (content: string) => void;
  /** Optional callback for "Quiz Me" */
  onQuiz?: (content: string) => void;
}

export function BookReaderView({
  bookTitle,
  onBack,
  onExplain,
  onQuiz,
}: BookReaderViewProps) {
  const [chunks, setChunks] = useState<BookChunk[]>([]);
  const [currentPage, setCurrentPage] = useState<number>(() =>
    loadSavedPage(bookTitle),
  );
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [bookmarks, setBookmarks] = useState<
    { page: number; note: string; createdAt: string }[]
  >(() => loadSavedBookmarks(bookTitle));
  const [showBookmarkInput, setShowBookmarkInput] = useState(false);
  const [bookmarkNote, setBookmarkNote] = useState("");
  const [showBookmarksPanel, setShowBookmarksPanel] = useState(false);
  const [quizPlaceholder, setQuizPlaceholder] = useState(false);

  const fetchKeyRef = useRef(bookTitle);

  // ── Load chunks ──

  useEffect(() => {
    let cancelled = false;
    fetchKeyRef.current = bookTitle;

    invoke<GetBookChunksResponse>("cmd_get_book_chunks", {
      request: { source: bookTitle },
    })
      .then((r) => {
        if (!cancelled && fetchKeyRef.current === bookTitle) {
          setChunks(r.chunks);
          setError(null);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled && fetchKeyRef.current === bookTitle) {
          setError(e instanceof Error ? e.message : String(e));
        }
      })
      .finally(() => {
        if (!cancelled && fetchKeyRef.current === bookTitle) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [bookTitle]);

  // ── Save progress on page change ──

  const goToPage = useCallback(
    (page: number) => {
      if (chunks.length === 0) return;
      const clamped = Math.max(0, Math.min(page, chunks.length - 1));
      setCurrentPage(clamped);
      try {
        localStorage.setItem(progressKey(bookTitle), clamped.toString());
      } catch {
        // ignore
      }
    },
    [chunks.length, bookTitle],
  );

  // ── Add bookmark ──

  const addBookmark = useCallback(() => {
    const newBookmark = {
      page: currentPage,
      note: bookmarkNote.trim() || `Page ${currentPage + 1}`,
      createdAt: new Date().toISOString(),
    };
    const updated = [...bookmarks, newBookmark];
    setBookmarks(updated);
    try {
      localStorage.setItem(bookmarkKey(bookTitle), JSON.stringify(updated));
    } catch {
      // ignore
    }
    setBookmarkNote("");
    setShowBookmarkInput(false);
  }, [currentPage, bookmarkNote, bookmarks, bookTitle]);

  // ── Remove bookmark ──

  const removeBookmark = useCallback(
    (idx: number) => {
      const updated = bookmarks.filter((_, i) => i !== idx);
      setBookmarks(updated);
      try {
        localStorage.setItem(bookmarkKey(bookTitle), JSON.stringify(updated));
      } catch {
        // ignore
      }
    },
    [bookmarks, bookTitle],
  );

  // ── Computed ──

  const totalPages = chunks.length;
  const progressPct =
    totalPages > 0 ? Math.round(((currentPage + 1) / totalPages) * 100) : 0;
  const currentChunk = chunks[currentPage];

  // ── Loading / Error states ──

  if (loading) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-4">
        <div className="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin mb-3" />
        <p className="text-sm text-muted-foreground">
          Loading &ldquo;{bookTitle}&rdquo;
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-4 text-center max-w-sm mx-auto">
        <BookOpen className="h-12 w-12 text-muted-foreground/30 mb-4" />
        <p className="text-lg font-medium text-destructive mb-2">
          Failed to Load Book
        </p>
        <p className="text-sm text-muted-foreground mb-4">{error}</p>
        <Button
          variant="outline"
          onClick={onBack}
          icon={<ArrowLeft className="h-4 w-4" />}
        >
          Back to Library
        </Button>
      </div>
    );
  }

  if (totalPages === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-4 text-center max-w-sm mx-auto">
        <BookOpen className="h-12 w-12 text-muted-foreground/30 mb-4" />
        <p className="text-lg font-medium text-muted-foreground mb-2">
          No Pages Found
        </p>
        <p className="text-sm text-muted-foreground mb-4">
          No knowledge chunks were found for &ldquo;{bookTitle}&rdquo;.
          <br />
          Try running the ingestion pipeline first.
        </p>
        <Button
          variant="outline"
          onClick={onBack}
          icon={<ArrowLeft className="h-4 w-4" />}
        >
          Back to Library
        </Button>
      </div>
    );
  }

  // ── Render ──

  return (
    <div className="h-full flex flex-col p-5">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 shrink-0">
        <Button
          variant="ghost"
          size="sm"
          onClick={onBack}
          icon={<ArrowLeft className="h-4 w-4" />}
        >
          Back
        </Button>
        <div className="flex flex-col items-center min-w-0 px-4">
          <h2 className="text-sm font-semibold text-foreground truncate max-w-56">
            {bookTitle}
          </h2>
          <p className="text-[11px] text-muted-foreground">
            Page {currentPage + 1} / {totalPages}
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowBookmarksPanel((p) => !p)}
          icon={
            showBookmarksPanel ? (
              <X className="h-4 w-4" />
            ) : (
              <Bookmark className="h-4 w-4" />
            )
          }
        >
          {bookmarks.length > 0 && (
            <Badge
              variant="default"
              className="ml-1 h-4 min-w-4 px-1 text-[10px]"
            >
              {bookmarks.length}
            </Badge>
          )}
        </Button>
      </div>

      {/* Bookmarks Panel */}
      {showBookmarksPanel && bookmarks.length > 0 && (
        <Card className="mb-3 shrink-0 border-border bg-card">
          <div className="px-4 py-3 border-b border-border flex items-center justify-between">
            <span className="text-xs font-medium text-muted-foreground flex items-center gap-1.5">
              <Bookmark className="h-3.5 w-3.5" />
              Bookmarks ({bookmarks.length})
            </span>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-[11px]"
              onClick={() => setShowBookmarksPanel(false)}
            >
              Collapse
            </Button>
          </div>
          <ScrollArea className="max-h-32">
            <div className="px-2 py-1">
              {bookmarks.map((bm, i) => (
                <div
                  key={i}
                  className="flex items-center justify-between px-2 py-1.5 rounded-md hover:bg-muted/50 transition-colors"
                >
                  <button
                    onClick={() => goToPage(bm.page)}
                    className="text-xs text-primary hover:underline bg-transparent border-0 cursor-pointer text-left flex-1 min-w-0 flex items-center gap-2"
                  >
                    <span className="text-[10px] font-mono text-muted-foreground shrink-0">
                      {bm.page + 1}
                    </span>
                    <span className="truncate">{bm.note}</span>
                  </button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0 text-muted-foreground hover:text-destructive"
                    onClick={() => removeBookmark(i)}
                    title="Remove bookmark"
                  >
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              ))}
            </div>
          </ScrollArea>
        </Card>
      )}

      {/* Content Area */}
      <Card className="flex-1 overflow-hidden border-border bg-card mb-4">
        <ScrollArea className="h-full">
          <div className="max-w-3xl mx-auto p-8">
            <div className="text-sm text-foreground leading-loose whitespace-pre-wrap font-sans select-text">
              {currentChunk?.content ?? "(empty page)"}
            </div>

            {currentChunk?.position_fen && (
              <div className="mt-6 pt-4 border-t border-border">
                <span className="text-[10px] text-muted-foreground font-mono">
                  FEN: {currentChunk.position_fen}
                </span>
              </div>
            )}

            {currentChunk && (
              <div className="mt-3">
                <Badge variant="secondary" className="text-[10px]">
                  {currentChunk.chunk_type}
                </Badge>
              </div>
            )}
          </div>
        </ScrollArea>
      </Card>

      {/* Progress Bar */}
      <div className="flex items-center gap-3 mb-3 shrink-0">
        <Button
          variant="outline"
          size="sm"
          onClick={() => goToPage(currentPage - 1)}
          disabled={currentPage <= 0}
          icon={<ChevronLeft className="h-4 w-4" />}
        >
          Prev
        </Button>

        <div className="flex-1 flex flex-col gap-1">
          <Progress value={progressPct} />
        </div>

        <span className="text-xs text-muted-foreground font-mono w-10 text-right whitespace-nowrap">
          {progressPct}%
        </span>

        <Button
          variant="outline"
          size="sm"
          onClick={() => goToPage(currentPage + 1)}
          disabled={currentPage >= totalPages - 1}
          icon={<ChevronRight className="h-4 w-4" />}
        >
          Next
        </Button>
      </div>

      {/* Actions */}
      <div className="flex gap-2 shrink-0">
        <Button
          variant="outline"
          size="sm"
          className="flex-1"
          onClick={() => onExplain?.(currentChunk?.content ?? "")}
          disabled={!currentChunk}
          icon={<Lightbulb className="h-3.5 w-3.5" />}
        >
          Explain This
        </Button>

        <Button
          variant="outline"
          size="sm"
          className="flex-1"
          onClick={() => {
            if (onQuiz) {
              onQuiz(currentChunk?.content ?? "");
            } else {
              setQuizPlaceholder(true);
              setTimeout(() => setQuizPlaceholder(false), 2000);
            }
          }}
          disabled={!currentChunk}
          icon={<BrainCircuit className="h-3.5 w-3.5" />}
        >
          {quizPlaceholder ? "Coming soon!" : "Quiz Me"}
        </Button>

        <Button
          variant="outline"
          size="sm"
          className="flex-1"
          onClick={() => setShowBookmarkInput(!showBookmarkInput)}
          disabled={!currentChunk}
          icon={
            showBookmarkInput ? (
              <Check className="h-3.5 w-3.5" />
            ) : (
              <Bookmark className="h-3.5 w-3.5" />
            )
          }
        >
          {showBookmarkInput ? "Save" : "Bookmark"}
        </Button>
      </div>

      {/* Bookmark Note Input */}
      {showBookmarkInput && (
        <div className="mt-2 flex gap-2 shrink-0">
          <input
            value={bookmarkNote}
            onChange={(e) => setBookmarkNote(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") addBookmark();
              if (e.key === "Escape") setShowBookmarkInput(false);
            }}
            placeholder="Add a note..."
            autoFocus
            className={cn(
              "flex-1 rounded-lg border border-border bg-background px-3 py-2 text-xs text-foreground shadow-sm",
              "placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            )}
          />
          <Button variant="default" size="sm" onClick={addBookmark}>
            Save
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowBookmarkInput(false)}
          >
            Cancel
          </Button>
        </div>
      )}
    </div>
  );
}
