import type { Transport } from "../transport";
import type { ClipboardSnapshot, ClipboardEntry } from "../types";

const EXT = "clipboard";

export interface ClipboardApi {
  get(): Promise<ClipboardSnapshot>;
  setText(text: string): Promise<void>;
  setImage(imagePath: string): Promise<void>;
  setFiles(files: string[]): Promise<void>;
  listHistory(limit?: number): Promise<ClipboardEntry[]>;
  clearHistory(): Promise<void>;
}

export function createClipboard(t: Transport): ClipboardApi {
  return {
    async get() {
      return t.invokeTool<ClipboardSnapshot>(EXT, "get_clipboard");
    },
    async setText(text) {
      await t.invokeTool(EXT, "set_clipboard", { text });
    },
    async setImage(imagePath) {
      await t.invokeTool(EXT, "set_clipboard", { image: imagePath });
    },
    async setFiles(files) {
      await t.invokeTool(EXT, "set_clipboard", { files });
    },
    async listHistory(limit?) {
      const input: Record<string, unknown> = {};
      if (limit !== undefined) input.limit = limit;
      return t.invokeTool<ClipboardEntry[]>(EXT, "list_clipboard_history", input);
    },
    async clearHistory() {
      await t.invokeTool(EXT, "clear_clipboard_history");
    },
  };
}
