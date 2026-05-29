import type { StateCreator } from "zustand";
import type {
  AppState,
  Repertoire,
  RepertoireChapter,
  RepertoireEntry,
} from "./index";

export interface RepertoireSlice {
  repertoires: Repertoire[];
  repertoireChapters: Record<string, RepertoireChapter>;
  repertoireEntries: Record<string, RepertoireEntry[]>;
  activeRepertoireId: string | null;
  createRepertoire: (name: string, color: "white" | "black") => string;
  addChapter: (
    repertoireId: string,
    name: string,
    parentId?: string,
  ) => string;
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

export const createRepertoireSlice: StateCreator<
  AppState,
  [],
  [],
  RepertoireSlice
> = (set, get) => ({
  repertoires: [],
  repertoireChapters: {},
  repertoireEntries: {},
  activeRepertoireId: null,

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
    const state = get();
    const repertoire = state.repertoires.find((r) => r.id === repertoireId);
    if (!repertoire) return "";

    const chapters: string[] = [];
    for (const chapterId of repertoire.chapters) {
      const chapter = state.repertoireChapters[chapterId];
      if (!chapter) continue;
      const entries = state.repertoireEntries[chapterId] || [];
      if (entries.length === 0) continue;

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
});
