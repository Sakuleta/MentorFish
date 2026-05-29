// ─── Application Store (Zustand) ───
//
// Central state management for the MentorFish UI.
// Composed from focused domain slices via Zustand's slice pattern.

import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

// ─── Re-export slice types ───

export type { BoardSlice } from "./boardStore";
export type { PlaySlice } from "./playStore";
export type { SettingsSlice } from "./settingsStore";
export type { RepertoireSlice } from "./repertoireStore";
export type { UiSlice } from "./uiStore";

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

// ─── Slice imports ───

import { createBoardSlice, type BoardSlice } from "./boardStore";
import { createPlaySlice, type PlaySlice } from "./playStore";
import { createSettingsSlice, type SettingsSlice } from "./settingsStore";
import {
  createRepertoireSlice,
  type RepertoireSlice,
} from "./repertoireStore";
import { createUiSlice, type UiSlice } from "./uiStore";

// ─── Composed Application State ───

export type AppState = BoardSlice & PlaySlice & SettingsSlice & RepertoireSlice & UiSlice;

// ─── Store ───

export const useAppStore = create<AppState>()(
  persist(
    (...args) => ({
      ...createBoardSlice(...args),
      ...createPlaySlice(...args),
      ...createSettingsSlice(...args),
      ...createRepertoireSlice(...args),
      ...createUiSlice(...args),
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
