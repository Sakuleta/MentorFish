// ─── Tauri Bridge Safety ───
//
// Polls for the Tauri IPC bridge to become available before calling any API.
// Prevents "Cannot read properties of undefined (reading 'invoke')" errors
// that happen when React renders before the WebView injects __TAURI_INTERNALS__.

type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type ListenFn = <T>(
  event: string,
  handler: (payload: { payload: T }) => void,
) => Promise<() => void>;

interface TauriAPI {
  invoke: InvokeFn;
  listen: ListenFn;
  ready: boolean;
}

let cachedAPI: TauriAPI | null = null;

/**
 * Wait for Tauri's __TAURI_INTERNALS__ to be injected into the page.
 * Returns true if available within `timeoutMs` (default 5000ms).
 */
async function waitForBridge(timeoutMs = 5000): Promise<boolean> {
  if (typeof window === "undefined") return false;
  const win = window as unknown as {
    __TAURI_INTERNALS__?: { invoke?: unknown };
  };
  if (win.__TAURI_INTERNALS__?.invoke) return true;

  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    await new Promise((r) => setTimeout(r, 50));
    if (win.__TAURI_INTERNALS__?.invoke) return true;
  }
  return false;
}

/**
 * Get the Tauri API (invoke + listen) once the bridge is ready.
 * Returns null if Tauri is not available (e.g. running in a browser).
 */
import type { MakeMoveRequest, MakeMoveResponse } from "./types";

export async function makeMove(
  req: MakeMoveRequest,
): Promise<MakeMoveResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_make_move", { request: req });
}

export async function aiMove(
  fen: string,
  strengthMode?: string,
  targetElo?: number,
): Promise<MakeMoveResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_ai_move", {
    fen,
    strength_mode: strengthMode,
    target_elo: targetElo,
  });
}

// ─── Typed IPC wrappers ───
// Components should use these instead of calling invoke() directly.

import type {
  AnalyzePositionResponse,
  UserProfileResponse,
  GameSummary,
  GetBookChunksResponse,
  IngestionReportResponse,
  KnowledgeSummaryResponse,
} from "./types";

export async function analyzePosition(args: {
  fen: string;
  depth?: number;
  pipeline_type?: string;
  game_id?: string;
  moves?: unknown[];
}): Promise<AnalyzePositionResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_analyze_position", args);
}

export async function getUserProfile(): Promise<UserProfileResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_get_user_profile");
}

export async function getRecentGames(): Promise<GameSummary[]> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_get_recent_games");
}

export async function getBookChunks(source: string): Promise<GetBookChunksResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_get_book_chunks", { request: { source } });
}

export async function runIngestion(): Promise<IngestionReportResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_run_ingestion");
}

export async function getKnowledgeSummary(): Promise<KnowledgeSummaryResponse> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_get_knowledge_summary");
}

export async function reportError(args: {
  message: string;
  stack: string | null;
  component: string | null;
  timestamp: string;
}): Promise<void> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  return tauri.invoke("cmd_report_error", args);
}

export async function getChannel(): Promise<{
  Channel: new <T>(onmessage?: (response: T) => void) => unknown;
}> {
  const tauri = await getTauri();
  if (!tauri) throw new Error("Tauri bridge not available");
  // Channel is imported dynamically from @tauri-apps/api/core
  const { Channel } = await import("@tauri-apps/api/core");
  return { Channel };
}

export async function getTauri(): Promise<TauriAPI | null> {
  // Return cached result if bridge is already known to be ready
  if (cachedAPI?.ready) return cachedAPI;

  const available = await waitForBridge();
  if (!available) {
    console.warn(
      "[MentorFish] Tauri bridge not available — backend features disabled",
    );
    return null;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");

  cachedAPI = {
    invoke: invoke as InvokeFn,
    listen: listen as unknown as ListenFn,
    ready: true,
  };
  return cachedAPI;
}
