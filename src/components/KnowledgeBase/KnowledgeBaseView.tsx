// ─── Knowledge Base View ───
// PDF upload, PGN import, ingestion status, corpus inventory.
// Section 11.4 of the PRD.

import { useEffect, useState, useRef } from "react";
import {
  BookOpen,
  Layers,
  Database,
  FileText,
  Gamepad2,
  Upload,
  Check,
  Minus,
  RotateCw,
  AlertCircle,
} from "lucide-react";
import { getTauri } from "../../lib/tauriBridge";
import type {
  KnowledgeSummaryResponse,
  IngestionReportResponse,
} from "../../lib/types";
import { cn } from "../../lib/utils";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  Button,
  Badge,
  Separator,
  Spinner,
} from "../../components/ui";

// ─── Constants ───

const CHUNK_TYPE_COLORS: Record<string, string> = {
  endgame: "bg-success/15 text-success",
  endgame_technique: "bg-success/15 text-success",
  opening: "bg-primary/15 text-primary",
  tactics: "bg-destructive/15 text-destructive",
  motif: "bg-destructive/15 text-destructive",
  strategy: "bg-accent/15 text-accent",
  concept: "bg-accent/15 text-accent",
  games: "bg-muted/30 text-muted-foreground",
  training: "bg-warning/15 text-warning",
  instructive_example: "bg-warning/15 text-warning",
};

function chunkTypeLabel(t: string): string {
  const map: Record<string, string> = {
    endgame: "endgame",
    endgame_technique: "endgame",
    opening: "opening",
    tactics: "tactics",
    motif: "tactics",
    strategy: "strategy",
    concept: "strategy",
    games: "games",
    training: "training",
    instructive_example: "training",
  };
  return map[t] ?? t;
}

// ─── Component ───

export function KnowledgeBaseView() {
  // ── Summary state ──
  const [summary, setSummary] = useState<KnowledgeSummaryResponse | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(true);
  const [summaryError, setSummaryError] = useState<string | null>(null);

  // ── Ingestion state ──
  const [ingesting, setIngesting] = useState(false);
  const [lastReport, setLastReport] = useState<IngestionReportResponse | null>(
    null,
  );
  const [lastRunTime, setLastRunTime] = useState<string | null>(null);

  // ── Import state ──
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const pdfInputRef = useRef<HTMLInputElement>(null);
  const pgnInputRef = useRef<HTMLInputElement>(null);

  // ── Load summary on mount ──

  const loadSummary = async () => {
    setSummaryLoading(true);
    setSummaryError(null);
    try {
      const tauri = await getTauri();
      if (!tauri) {
        setSummaryError("Backend not available");
        return;
      }
      const data = await tauri.invoke<KnowledgeSummaryResponse>(
        "cmd_get_knowledge_summary",
      );
      setSummary(data);
    } catch (e) {
      setSummaryError(String(e));
    } finally {
      setSummaryLoading(false);
    }
  };

  useEffect(() => {
    let cancelled = false;
    const fetch = async () => {
      setSummaryLoading(true);
      setSummaryError(null);
      try {
        const tauri = await getTauri();
        if (cancelled) return;
        if (!tauri) {
          setSummaryError("Backend not available");
          return;
        }
        const data = await tauri.invoke<KnowledgeSummaryResponse>(
          "cmd_get_knowledge_summary",
        );
        if (!cancelled) setSummary(data);
      } catch (e) {
        if (!cancelled) setSummaryError(String(e));
      } finally {
        if (!cancelled) setSummaryLoading(false);
      }
    };
    fetch();
    return () => {
      cancelled = true;
    };
  }, []);

  // ── Run ingestion ──

  const handleRunIngestion = async () => {
    setIngesting(true);
    try {
      const tauri = await getTauri();
      if (!tauri) return;
      const report =
        await tauri.invoke<IngestionReportResponse>("cmd_run_ingestion");
      setLastReport(report);
      setLastRunTime(new Date().toLocaleString());
      await loadSummary();
    } catch (e) {
      setLastReport({
        books_processed: 0,
        chunks_created: 0,
        chunks_embedded: 0,
        message: `Error: ${String(e)}`,
      });
    } finally {
      setIngesting(false);
    }
  };

  // ── File import (PDF) ──

  const handleImportPdf = () => {
    pdfInputRef.current?.click();
  };

  const handlePdfSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    await importFile(file, "pdf");
    if (pdfInputRef.current) pdfInputRef.current.value = "";
  };

  // ── File import (PGN) ──

  const handleImportPgn = () => {
    pgnInputRef.current?.click();
  };

  const handlePgnSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    await importFile(file, "pgn");
    if (pgnInputRef.current) pgnInputRef.current.value = "";
  };

  const importFile = async (file: File, fileType: "pdf" | "pgn") => {
    setImportStatus(`Importing ${file.name}...`);
    try {
      const tauri = await getTauri();
      if (!tauri) {
        setImportStatus("Backend not available");
        return;
      }
      const buffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));

      const dest = await tauri.invoke<string>("cmd_copy_to_knowledge", {
        fileName: file.name,
        fileContent: bytes,
        fileType,
      });
      setImportStatus(`Imported: ${file.name} -> ${dest}`);
    } catch (e) {
      setImportStatus(`Import failed: ${String(e)}`);
    }
  };

  // ── Render ──

  const totalBooks = summary?.total_books ?? 0;
  const totalChunks = summary?.total_chunks ?? 0;
  const totalEmbedded = summary?.total_embedded ?? 0;

  return (
    <div className="h-full flex flex-col p-5 overflow-auto">
      {/* Header */}
      <div className="mb-6 shrink-0">
        <h2 className="text-xl font-semibold tracking-tight text-foreground">
          Knowledge Base
        </h2>
        <p className="text-xs text-muted-foreground mt-0.5">
          Manage your chess library and training data
        </p>
      </div>

      {/* Stats cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-6 shrink-0">
        <Card className="border-border bg-card">
          <CardContent className="flex items-center gap-4 p-5">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <BookOpen className="h-5 w-5 text-primary" />
            </div>
            <div>
              <p className="text-2xl font-semibold text-foreground tabular-nums">
                {totalBooks}
              </p>
              <p className="text-xs text-muted-foreground">Books</p>
            </div>
          </CardContent>
        </Card>
        <Card className="border-border bg-card">
          <CardContent className="flex items-center gap-4 p-5">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent/10">
              <Layers className="h-5 w-5 text-accent" />
            </div>
            <div>
              <p className="text-2xl font-semibold text-foreground tabular-nums">
                {totalChunks.toLocaleString()}
              </p>
              <p className="text-xs text-muted-foreground">Chunks</p>
            </div>
          </CardContent>
        </Card>
        <Card className="border-border bg-card">
          <CardContent className="flex items-center gap-4 p-5">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-success/10">
              <Database className="h-5 w-5 text-success" />
            </div>
            <div>
              <p className="text-2xl font-semibold text-foreground tabular-nums">
                {totalEmbedded.toLocaleString()}
              </p>
              <p className="text-xs text-muted-foreground">Embedded</p>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="flex flex-col gap-6 flex-1 min-h-0">
        {/* Corpus Inventory */}
        <Card className="border-border bg-card flex flex-col min-h-0">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <BookOpen className="h-4 w-4 text-primary" />
              Corpus Inventory
            </CardTitle>
            <CardDescription>
              {totalBooks} source{totalBooks !== 1 ? "s" : ""} indexed
            </CardDescription>
          </CardHeader>
          <CardContent className="flex-1 overflow-auto min-h-0">
            {summaryLoading ? (
              <div className="flex items-center justify-center py-8">
                <Spinner size="md" className="mr-2" />
                <span className="text-sm text-muted-foreground">
                  Loading inventory...
                </span>
              </div>
            ) : summaryError ? (
              <div className="py-4 text-center">
                <AlertCircle className="h-8 w-8 text-destructive mx-auto mb-2" />
                <p className="text-sm text-destructive mb-3">{summaryError}</p>
                <Button variant="outline" size="sm" onClick={loadSummary}>
                  Retry
                </Button>
              </div>
            ) : summary && summary.books.length > 0 ? (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="border-b border-border text-muted-foreground">
                      <th className="text-left py-2 px-2 font-medium">
                        Source
                      </th>
                      <th className="text-right py-2 px-2 font-medium">
                        Chunks
                      </th>
                      <th className="text-left py-2 px-2 font-medium">Type</th>
                      <th className="text-center py-2 px-2 font-medium w-16">
                        Status
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {summary.books.map((book) => (
                      <tr
                        key={book.title}
                        className="border-b border-border/50 hover:bg-muted/40 transition-colors"
                      >
                        <td className="py-2 px-2 text-foreground truncate max-w-72">
                          {book.title}
                        </td>
                        <td className="py-2 px-2 text-right text-muted-foreground tabular-nums">
                          {book.chunk_count.toLocaleString()}
                        </td>
                        <td className="py-2 px-2">
                          <Badge
                            variant="outline"
                            className={cn(
                              "text-[10px]",
                              CHUNK_TYPE_COLORS[book.chunk_type] ??
                                "bg-muted/30 text-muted-foreground",
                            )}
                          >
                            {chunkTypeLabel(book.chunk_type)}
                          </Badge>
                        </td>
                        <td className="py-2 px-2 text-center">
                          {book.has_embeddings ? (
                            <Check className="h-3.5 w-3.5 text-success mx-auto" />
                          ) : (
                            <Minus className="h-3.5 w-3.5 text-muted-foreground/40 mx-auto" />
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <div className="mt-3 pt-3 border-t border-border flex items-center justify-between text-xs text-muted-foreground">
                  <span>
                    Total: {summary.total_books} book
                    {summary.total_books !== 1 ? "s" : ""},{" "}
                    {summary.total_chunks.toLocaleString()} chunks
                  </span>
                  <span>
                    {summary.total_embedded.toLocaleString()} embedded
                  </span>
                </div>
              </div>
            ) : (
              <div className="py-8 text-center">
                <BookOpen className="h-10 w-10 text-muted-foreground/30 mx-auto mb-3" />
                <p className="text-sm text-muted-foreground mb-1">
                  No knowledge base yet.
                </p>
                <p className="text-xs text-muted-foreground">
                  Run ingestion to get started.
                </p>
              </div>
            )}
          </CardContent>
        </Card>

        <Separator />

        {/* Ingestion Pipeline */}
        <Card className="border-border bg-card shrink-0">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <RotateCw className="h-4 w-4 text-primary" />
              Ingestion Pipeline
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 mb-4">
              <Button
                variant="default"
                size="sm"
                onClick={handleRunIngestion}
                disabled={ingesting}
                icon={
                  ingesting ? (
                    <Spinner size="sm" />
                  ) : (
                    <RotateCw className="h-3.5 w-3.5" />
                  )
                }
              >
                {ingesting ? "Ingesting..." : "Run Full Ingestion"}
              </Button>

              <div className="flex items-center gap-2">
                <span className="text-xs text-muted-foreground">Status:</span>
                <Badge
                  variant={
                    ingesting ? "warning" : lastReport ? "success" : "secondary"
                  }
                  className="text-[10px]"
                >
                  {ingesting ? "Running..." : lastReport ? "Complete" : "Idle"}
                </Badge>
              </div>
            </div>

            <div className="text-xs text-muted-foreground space-y-1">
              <p>
                Last run:{" "}
                <span className="text-foreground">
                  {lastRunTime ?? "Never"}
                </span>
              </p>
              {lastReport ? (
                <>
                  <p>
                    Results:{" "}
                    <span className="text-foreground">
                      {lastReport.books_processed} books,{" "}
                      {lastReport.chunks_created.toLocaleString()} chunks,{" "}
                      {lastReport.chunks_embedded.toLocaleString()} embedded
                    </span>
                  </p>
                  <p className="italic mt-1 text-muted-foreground/80">
                    {lastReport.message}
                  </p>
                </>
              ) : (
                <p>
                  Results:{" "}
                  <span className="text-foreground">
                    0 books, 0 chunks, 0 embedded
                  </span>
                </p>
              )}
            </div>
          </CardContent>
        </Card>

        <Separator />

        {/* Import Files */}
        <Card className="border-border bg-card shrink-0">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Upload className="h-4 w-4 text-primary" />
              Import Files
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
              <input
                ref={pdfInputRef}
                type="file"
                accept=".pdf"
                onChange={handlePdfSelected}
                className="hidden"
              />
              <input
                ref={pgnInputRef}
                type="file"
                accept=".pgn"
                onChange={handlePgnSelected}
                className="hidden"
              />

              <Card className="border-border/60 bg-background hover:border-primary/30 transition-colors">
                <CardContent className="flex items-center gap-4 p-4">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-destructive/10">
                    <FileText className="h-5 w-5 text-destructive" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-foreground">
                      PDF Books
                    </p>
                    <p className="text-[11px] text-muted-foreground">
                      knowledge/books/
                    </p>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleImportPdf}
                    icon={<Upload className="h-3.5 w-3.5" />}
                  >
                    Upload
                  </Button>
                </CardContent>
              </Card>

              <Card className="border-border/60 bg-background hover:border-primary/30 transition-colors">
                <CardContent className="flex items-center gap-4 p-4">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-success/10">
                    <Gamepad2 className="h-5 w-5 text-success" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-foreground">
                      PGN Games
                    </p>
                    <p className="text-[11px] text-muted-foreground">
                      knowledge/pgn/
                    </p>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleImportPgn}
                    icon={<Upload className="h-3.5 w-3.5" />}
                  >
                    Upload
                  </Button>
                </CardContent>
              </Card>
            </div>

            {importStatus && (
              <div
                className={cn(
                  "px-3 py-2 rounded-lg text-xs border",
                  importStatus.startsWith("Imported:")
                    ? "bg-success/10 text-success border-success/20"
                    : importStatus.startsWith("Importing")
                      ? "bg-primary/10 text-primary border-primary/20"
                      : "bg-destructive/10 text-destructive border-destructive/20",
                )}
              >
                {importStatus}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
