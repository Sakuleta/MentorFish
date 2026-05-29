import type { StateCreator } from "zustand";
import type { AppState } from "./index";
import type { CoachingAlert, EngineProgress } from "./index";

export interface BoardSlice {
  currentFen: string;
  setCurrentFen: (fen: string) => void;
  boardOrientation: "white" | "black";
  setBoardOrientation: (orientation: "white" | "black") => void;
  lastExplanation: string | null;
  engineEval: number | null;
  isStreaming: boolean;
  streamingTokens: string;
  coachingAlert: CoachingAlert | null;
  currentAgent: string | null;
  engineProgress: EngineProgress | null;
  setAnalysisResult: (explanation: string, evalCp: number) => void;
  appendStreamToken: (token: string) => void;
  setStreaming: (active: boolean) => void;
  setCoachingAlert: (alert: CoachingAlert | null) => void;
  setEngineProgress: (progress: EngineProgress | null) => void;
  resetAnalysis: () => void;
}

export const createBoardSlice: StateCreator<AppState, [], [], BoardSlice> = (
  set,
) => ({
  currentFen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
  setCurrentFen: (fen) => set({ currentFen: fen }),

  boardOrientation: "white",
  setBoardOrientation: (orientation) => set({ boardOrientation: orientation }),

  lastExplanation: null,
  engineEval: null,

  isStreaming: false,
  streamingTokens: "",

  coachingAlert: null,
  currentAgent: null,

  engineProgress: null,

  setAnalysisResult: (explanation, evalCp) =>
    set({
      lastExplanation: explanation,
      engineEval: evalCp,
      isStreaming: false,
    }),

  appendStreamToken: (token) =>
    set((state) => ({
      streamingTokens: state.streamingTokens + token,
    })),

  setStreaming: (active) =>
    set((state) => ({
      isStreaming: active,
      streamingTokens: active ? state.streamingTokens : "",
    })),

  setCoachingAlert: (alert) =>
    set({
      coachingAlert: alert,
      currentAgent: alert?.type ?? null,
    }),

  setEngineProgress: (progress) => set({ engineProgress: progress }),

  resetAnalysis: () =>
    set({
      lastExplanation: null,
      engineEval: null,
      streamingTokens: "",
      isStreaming: false,
      coachingAlert: null,
      currentAgent: null,
      engineProgress: null,
    }),
});
