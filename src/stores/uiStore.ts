import type { StateCreator } from "zustand";
import type { StudyPlan } from "../lib/types";
import type { AppState, View } from "./index";

export interface UiSlice {
  activeView: View;
  setActiveView: (view: View) => void;
  engineStatus: boolean;
  inferenceStatus: boolean;
  databaseStatus: boolean;
  setHealthStatus: (
    engine: boolean,
    inference: boolean,
    database: boolean,
  ) => void;
  curriculumPlan: StudyPlan | null;
  completedSessions: string[];
  isGeneratingPlan: boolean;
  setCurriculumPlan: (plan: StudyPlan | null) => void;
  toggleSessionComplete: (sessionKey: string) => void;
  setIsGeneratingPlan: (generating: boolean) => void;
}

export const createUiSlice: StateCreator<AppState, [], [], UiSlice> = (
  set,
) => ({
  activeView: "board",
  setActiveView: (view) => set({ activeView: view }),

  engineStatus: false,
  inferenceStatus: false,
  databaseStatus: false,
  setHealthStatus: (engine, inference, database) =>
    set({
      engineStatus: engine,
      inferenceStatus: inference,
      databaseStatus: database,
    }),

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
});
