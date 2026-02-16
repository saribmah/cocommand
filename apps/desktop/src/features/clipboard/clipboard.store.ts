import { create } from "zustand";
import type { ExtensionInvokeFn } from "../extension/extension.types";
import type { ClipboardEntry } from "./clipboard.types";

export interface ClipboardState {
  entries: ClipboardEntry[];
  selectedEntryId: string | null;
  isLoading: boolean;
  error: string | null;

  fetchHistory: () => Promise<void>;
  selectEntry: (id: string | null) => void;
  copyToClipboard: (entry: ClipboardEntry) => Promise<void>;
  clearHistory: () => Promise<void>;
}

export type ClipboardStore = ReturnType<typeof createClipboardStore>;

export const createClipboardStore = (invoke: ExtensionInvokeFn) => {
  return create<ClipboardState>()((set) => ({
    entries: [],
    selectedEntryId: null,
    isLoading: false,
    error: null,

    fetchHistory: async () => {
      set({ isLoading: true, error: null });

      try {
        const entries = await invoke<ClipboardEntry[]>(
          "clipboard",
          "list_clipboard_history",
          { limit: 100 },
        );
        set({ entries, isLoading: false, error: null });
      } catch (error) {
        set({ entries: [], isLoading: false, error: String(error) });
      }
    },

    selectEntry: (id: string | null) => {
      set({ selectedEntryId: id });
    },

    copyToClipboard: async (entry: ClipboardEntry) => {
      try {
        const input: Record<string, unknown> = { kind: entry.kind };
        if (entry.kind === "text" && entry.text) {
          input.text = entry.text;
        } else if (entry.kind === "image" && entry.image_path) {
          input.imagePath = entry.image_path;
        } else if (entry.kind === "files" && entry.files) {
          input.files = entry.files;
        }
        await invoke("clipboard", "set_clipboard", input);
      } catch (error) {
        set({ error: String(error) });
      }
    },

    clearHistory: async () => {
      try {
        await invoke("clipboard", "clear_clipboard_history", {});
        set({ entries: [], selectedEntryId: null, error: null });
      } catch (error) {
        set({ error: String(error) });
      }
    },
  }));
};
