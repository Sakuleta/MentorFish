// ─── Application Store (Zustand) ───
//
// Central state management for the MentorFish UI.

import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import type { StudyPlan } from "../lib/types";

// ─── View State ───

export type View =
  | "board"
  | "analysis"
  | "explorer"
  | "dashboard"
  | "curriculum"
  | "knowledge"
  | "knowledgebase"
  | "settings";

// ─── Coaching Alert ───

export interface CoachingAlert {
  type: string; // agent name: "tactical", "strategic", etc.
  message: string;
}

// ─── Engine Progress ───

export interface EngineProgress {
  depth: number;
  evalCp: number;
  bestMove: string | null;
  nodes: number | null;
}

// ─── Game Record ───

export interface GameRecord {
  id: string;
  fen: string;
  moves: string[];
  result: string; // "1-0" | "0-1" | "1/2-1/2" | "*"
  playedAt: string; // ISO date string
  opening?: string;
  playerColor: "white" | "black";
}

// ─── Repertoire Types ───

export interface RepertoireChapter {
  id: string;
  name: string; // e.g. "Sicilian", "French"
  color: "white" | "black";
  parentId: string | null; // for nested chapters
  order: number;
}

export interface RepertoireEntry {
  id: string;
  chapterId: string;
  fen: string; // position before the move
  move: string; // UCI format
  san: string; // human-readable notation
  eco?: string; // ECO code
  openingName?: string;
  notes?: string;
  isMainLine: boolean;
  createdAt: string; // ISO date
}

export interface Repertoire {
  id: string;
  name: string; // e.g. "My White Repertoire"
  color: "white" | "black";
  chapters: string[]; // ordered list of chapter IDs
  createdAt: string;
  updatedAt: string;
}

// ─── Application State ───

interface AppState {
  // Navigation
  activeView: View;
  setActiveView: (view: View) => void;

  // Health status
  engineStatus: boolean;
  inferenceStatus: boolean;
  databaseStatus: boolean;

  // FEN for the current board position
  currentFen: string;
  setCurrentFen: (fen: string) => void;

  // Analysis results (from backend)
  lastExplanation: string | null;
  engineEval: number | null;

  // Streaming state
  isStreaming: boolean;
  streamingTokens: string;

  // Coaching alert (which agent is currently working)
  coachingAlert: CoachingAlert | null;
  currentAgent: string | null;

  // Engine progress
  engineProgress: EngineProgress | null;

  // Curriculum
  curriculumPlan: StudyPlan | null;
  completedSessions: string[];
  isGeneratingPlan: boolean;
  setCurriculumPlan: (plan: StudyPlan | null) => void;
  toggleSessionComplete: (sessionKey: string) => void;
  setIsGeneratingPlan: (generating: boolean) => void;

  // Persona — selected coach persona
  persona: string;
  setPersona: (persona: string) => void;

  // Play mode — how Stockfish plays
  playMode: "full" | "human" | "training";
  setPlayMode: (mode: "full" | "human" | "training") => void;

  // Play strength — ELO for human-like mode
  playStrength: number;
  setPlayStrength: (strength: number) => void;

  // Engine configuration
  engineThreads: number;
  engineHash: number;
  engineDepth: number;
  engineMultiPv: number;
  setEngineThreads: (threads: number) => void;
  setEngineHash: (hash: number) => void;
  setEngineDepth: (depth: number) => void;
  setEngineMultiPv: (multipv: number) => void;

  // Game history — list of completed games
  gameHistory: GameRecord[];
  addGameToHistory: (record: GameRecord) => void;

  // Game over state — result for post-game analysis
  gameResult: string | null;
  setGameResult: (result: string | null) => void;

  // Theme
  theme: "dark" | "light";
  setTheme: (theme: "dark" | "light") => void;
  toggleTheme: () => void;

  // Board orientation — persists across sessions
  boardOrientation: "white" | "black";
  setBoardOrientation: (orientation: "white" | "black") => void;

  // Actions
  setHealthStatus: (
    engine: boolean,
    inference: boolean,
    database: boolean,
  ) => void;
  setAnalysisResult: (explanation: string, evalCp: number) => void;
  appendStreamToken: (token: string) => void;
  setStreaming: (active: boolean) => void;
  setCoachingAlert: (alert: CoachingAlert | null) => void;
  setEngineProgress: (progress: EngineProgress | null) => void;
  resetAnalysis: () => void;

  // ─── Onboarding ───
  onboardingCompleted: boolean;
  setOnboardingCompleted: (completed: boolean) => void;
  userSkillLevel: "beginner" | "intermediate" | "advanced" | "expert";
  setUserSkillLevel: (
    level: "beginner" | "intermediate" | "advanced" | "expert",
  ) => void;
  trainingGoals: string[];
  setTrainingGoals: (goals: string[]) => void;

  // ─── Repertoire ───
  repertoires: Repertoire[];
  repertoireChapters: Record<string, RepertoireChapter>;
  repertoireEntries: Record<string, RepertoireEntry[]>;
  activeRepertoireId: string | null;
  createRepertoire: (name: string, color: "white" | "black") => string;
  addChapter: (repertoireId: string, name: string, parentId?: string) => string;
  addRepertoireEntry: (
    chapterId: string,
    fen: string,
    move: string,
    san: string,
    openingName?: string,
  ) => string;
  removeRepertoireEntry: (entryId: string) => void;
  reorderChapters: (repertoireId: string, chapterIds: string[]) => void;
  setActiveRepertoire: (id: string | null) => void;
  exportRepertoirePgn: (repertoireId: string) => string;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Navigation
      activeView: "board",
      setActiveView: (view) => set({ activeView: view }),

      // Health status
      engineStatus: false,
      inferenceStatus: false,
      databaseStatus: false,

      // Current position
      currentFen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
      setCurrentFen: (fen) => set({ currentFen: fen }),

      // Analysis
      lastExplanation: null,
      engineEval: null,

      // Streaming
      isStreaming: false,
      streamingTokens: "",

      // Coaching alert
      coachingAlert: null,
      currentAgent: null,

      // Engine progress
      engineProgress: null,

      // Curriculum
      curriculumPlan: null,
      completedSessions: [],
      isGeneratingPlan: false,
      setCurriculumPlan: (plan) => set({ curriculumPlan: plan }),
      toggleSessionComplete: (sessionKey) =>
        set((state) => {
          const alreadyCompleted = state.completedSessions.includes(sessionKey);
          return {
            completedSessions: alreadyCompleted
              ? state.completedSessions.filter((k) => k !== sessionKey)
              : [...state.completedSessions, sessionKey],
          };
        }),
      setIsGeneratingPlan: (generating) =>
        set({ isGeneratingPlan: generating }),

      // Persona
      persona: "ModernGM",
      setPersona: (persona) => set({ persona }),

      // Onboarding
      onboardingCompleted: false,
      setOnboardingCompleted: (completed) =>
        set({ onboardingCompleted: completed }),
      userSkillLevel: "intermediate",
      setUserSkillLevel: (level) => set({ userSkillLevel: level }),
      trainingGoals: [],
      setTrainingGoals: (goals) => set({ trainingGoals: goals }),

      // Play mode
      playMode: "full",
      setPlayMode: (mode) => set({ playMode: mode }),

      // Play strength
      playStrength: 1500,
      setPlayStrength: (strength) => set({ playStrength: strength }),

      // Engine configuration
      engineThreads:
        typeof navigator !== "undefined" && navigator.hardwareConcurrency
          ? Math.max(1, navigator.hardwareConcurrency - 2)
          : 4,
      engineHash: 2048,
      engineDepth: 22,
      engineMultiPv: 5,
      setEngineThreads: (threads) => set({ engineThreads: threads }),
      setEngineHash: (hash) => set({ engineHash: hash }),
      setEngineDepth: (depth) => set({ engineDepth: depth }),
      setEngineMultiPv: (multipv) => set({ engineMultiPv: multipv }),

      // Game history
      gameHistory: [],
      addGameToHistory: (record) =>
        set((state) => ({
          gameHistory: [...state.gameHistory, record],
        })),

      // Game result
      gameResult: null,
      setGameResult: (result) => set({ gameResult: result }),

      // Theme
      theme: "dark",
      setTheme: (theme) => set({ theme }),
      toggleTheme: () =>
        set((state) => ({
          theme: state.theme === "dark" ? "light" : "dark",
        })),

      // Board orientation
      boardOrientation: "white",
      setBoardOrientation: (orientation) =>
        set({ boardOrientation: orientation }),

      // Actions
      setHealthStatus: (engine, inference, database) =>
        set({
          engineStatus: engine,
          inferenceStatus: inference,
          databaseStatus: database,
        }),

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

      // ─── Repertoire state ───
      repertoires: [],
      repertoireChapters: {},
      repertoireEntries: {},
      activeRepertoireId: null,

      // ─── Repertoire actions ───

      createRepertoire: (name, color) => {
        const id = `rep-${Date.now()}`;
        set((state) => ({
          repertoires: [
            ...state.repertoires,
            {
              id,
              name,
              color,
              chapters: [],
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
            },
          ],
        }));
        return id;
      },

      addChapter: (repertoireId, name, parentId) => {
        const id = `ch-${Date.now()}`;
        set((state) => {
          const repertoire = state.repertoires.find(
            (r) => r.id === repertoireId,
          );
          const chapter: RepertoireChapter = {
            id,
            name,
            color: repertoire?.color ?? "white",
            parentId: parentId ?? null,
            order: Object.values(state.repertoireChapters).filter(
              (c) => c.parentId === (parentId ?? null),
            ).length,
          };
          return {
            repertoireChapters: {
              ...state.repertoireChapters,
              [id]: chapter,
            },
            repertoires: state.repertoires.map((r) =>
              r.id === repertoireId
                ? {
                    ...r,
                    chapters: [...r.chapters, id],
                    updatedAt: new Date().toISOString(),
                  }
                : r,
            ),
          };
        });
        return id;
      },

      addRepertoireEntry: (chapterId, fen, move, san, openingName) => {
        const id = `re-${Date.now()}`;
        set((state) => ({
          repertoireEntries: {
            ...state.repertoireEntries,
            [chapterId]: [
              ...(state.repertoireEntries[chapterId] || []),
              {
                id,
                chapterId,
                fen,
                move,
                san,
                openingName,
                isMainLine: true,
                createdAt: new Date().toISOString(),
              },
            ],
          },
        }));
        return id;
      },

      removeRepertoireEntry: (entryId) => {
        set((state) => {
          const newEntries: Record<string, RepertoireEntry[]> = {};
          for (const [chId, entries] of Object.entries(
            state.repertoireEntries,
          )) {
            newEntries[chId] = entries.filter((e) => e.id !== entryId);
          }
          return { repertoireEntries: newEntries };
        });
      },

      reorderChapters: (repertoireId, chapterIds) => {
        set((state) => ({
          repertoires: state.repertoires.map((r) =>
            r.id === repertoireId
              ? {
                  ...r,
                  chapters: chapterIds,
                  updatedAt: new Date().toISOString(),
                }
              : r,
          ),
        }));
      },

      setActiveRepertoire: (id) => set({ activeRepertoireId: id }),

      exportRepertoirePgn: (repertoireId) => {
        const state = useAppStore.getState();
        const repertoire = state.repertoires.find((r) => r.id === repertoireId);
        if (!repertoire) return "";

        const chapters: string[] = [];
        for (const chapterId of repertoire.chapters) {
          const chapter = state.repertoireChapters[chapterId];
          if (!chapter) continue;
          const entries = state.repertoireEntries[chapterId] || [];
          if (entries.length === 0) continue;

          // Build move list with move numbers
          let moveText = "";
          let moveNum = 1;
          for (let i = 0; i < entries.length; i++) {
            if (i % 2 === 0) {
              moveText += `${moveNum}. `;
            }
            moveText += entries[i].san + " ";
            if (i % 2 === 1) {
              moveNum++;
            }
          }

          chapters.push(`[Chapter "${chapter.name}"]\n${moveText}\n\n`);
        }

        return chapters.join("\n");
      },
    }),
    {
      name: "mentorfish-store",
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        activeView: state.activeView,
        currentFen: state.currentFen,
        completedSessions: state.completedSessions,
        curriculumPlan: state.curriculumPlan,
        persona: state.persona,
        playMode: state.playMode,
        playStrength: state.playStrength,
        boardOrientation: state.boardOrientation,
        gameHistory: state.gameHistory,
        repertoires: state.repertoires,
        repertoireChapters: state.repertoireChapters,
        repertoireEntries: state.repertoireEntries,
        activeRepertoireId: state.activeRepertoireId,
        theme: state.theme,
        onboardingCompleted: state.onboardingCompleted,
        userSkillLevel: state.userSkillLevel,
        trainingGoals: state.trainingGoals,
        engineThreads: state.engineThreads,
        engineHash: state.engineHash,
        engineDepth: state.engineDepth,
        engineMultiPv: state.engineMultiPv,
      }),
    },
  ),
);
