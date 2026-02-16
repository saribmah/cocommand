import { create } from "zustand";
import type { ExtensionInvokeFn } from "../extension/extension.types";
import type { ScreenshotEntry } from "./screenshot.types";

export interface ScreenshotState {
  screenshots: ScreenshotEntry[];
  selectedFilename: string | null;
  isLoading: boolean;
  error: string | null;

  fetchScreenshots: () => Promise<void>;
  selectScreenshot: (filename: string | null) => void;
  copyToClipboard: (entry: ScreenshotEntry) => Promise<void>;
  deleteScreenshot: (filename: string) => Promise<void>;
}

export type ScreenshotStore = ReturnType<typeof createScreenshotStore>;

export const createScreenshotStore = (invoke: ExtensionInvokeFn) => {
  return create<ScreenshotState>()((set, get) => ({
    screenshots: [],
    selectedFilename: null,
    isLoading: false,
    error: null,

    fetchScreenshots: async () => {
      set({ isLoading: true, error: null });

      try {
        const screenshots = await invoke<ScreenshotEntry[]>(
          "screenshot",
          "list_screenshots",
          { limit: 200 },
        );
        set({ screenshots, isLoading: false, error: null });
      } catch (error) {
        set({ screenshots: [], isLoading: false, error: String(error) });
      }
    },

    selectScreenshot: (filename: string | null) => {
      set({ selectedFilename: filename });
    },

    copyToClipboard: async (entry: ScreenshotEntry) => {
      try {
        await invoke("screenshot", "copy_screenshot_to_clipboard", {
          filename: entry.filename,
        });
      } catch (error) {
        set({ error: String(error) });
      }
    },

    deleteScreenshot: async (filename: string) => {
      try {
        await invoke("screenshot", "delete_screenshot", { filename });
        const { screenshots, selectedFilename } = get();
        const updated = screenshots.filter((s) => s.filename !== filename);
        set({
          screenshots: updated,
          selectedFilename:
            selectedFilename === filename ? null : selectedFilename,
        });
      } catch (error) {
        set({ error: String(error) });
      }
    },
  }));
};
