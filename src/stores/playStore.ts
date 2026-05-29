import type { StateCreator } from "zustand";
import type { AppState, GameRecord } from "./index";

export interface PlaySlice {
  playMode: "full" | "human" | "training";
  setPlayMode: (mode: "full" | "human" | "training") => void;
  playStrength: number;
  setPlayStrength: (strength: number) => void;
  gameHistory: GameRecord[];
  addGameToHistory: (record: GameRecord) => void;
  gameResult: string | null;
  setGameResult: (result: string | null) => void;
}

export const createPlaySlice: StateCreator<AppState, [], [], PlaySlice> = (
  set,
) => ({
  playMode: "full",
  setPlayMode: (mode) => set({ playMode: mode }),

  playStrength: 1500,
  setPlayStrength: (strength) => set({ playStrength: strength }),

  gameHistory: [],
  addGameToHistory: (record) =>
    set((state) => ({
      gameHistory: [...state.gameHistory, record],
    })),

  gameResult: null,
  setGameResult: (result) => set({ gameResult: result }),
});
