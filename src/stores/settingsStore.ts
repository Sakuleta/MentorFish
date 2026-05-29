import type { StateCreator } from "zustand";
import type { AppState } from "./index";

export interface SettingsSlice {
  engineThreads: number;
  engineHash: number;
  engineDepth: number;
  engineMultiPv: number;
  setEngineThreads: (threads: number) => void;
  setEngineHash: (hash: number) => void;
  setEngineDepth: (depth: number) => void;
  setEngineMultiPv: (multipv: number) => void;
  persona: string;
  setPersona: (persona: string) => void;
  theme: "dark" | "light";
  setTheme: (theme: "dark" | "light") => void;
  toggleTheme: () => void;
  onboardingCompleted: boolean;
  setOnboardingCompleted: (completed: boolean) => void;
  userSkillLevel: "beginner" | "intermediate" | "advanced" | "expert";
  setUserSkillLevel: (
    level: "beginner" | "intermediate" | "advanced" | "expert",
  ) => void;
  trainingGoals: string[];
  setTrainingGoals: (goals: string[]) => void;
}

export const createSettingsSlice: StateCreator<
  AppState,
  [],
  [],
  SettingsSlice
> = (set) => ({
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

  persona: "ModernGM",
  setPersona: (persona) => set({ persona }),

  theme: "dark",
  setTheme: (theme) => set({ theme }),
  toggleTheme: () =>
    set((state) => ({
      theme: state.theme === "dark" ? "light" : "dark",
    })),

  onboardingCompleted: false,
  setOnboardingCompleted: (completed) =>
    set({ onboardingCompleted: completed }),
  userSkillLevel: "intermediate",
  setUserSkillLevel: (level) => set({ userSkillLevel: level }),
  trainingGoals: [],
  setTrainingGoals: (goals) => set({ trainingGoals: goals }),
});
