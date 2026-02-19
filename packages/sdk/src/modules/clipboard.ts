import type { Client } from "../client";
import { invokeToolUnwrap } from "../client";
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

export function createClipboard(client: Client): ClipboardApi {
  return {
    async get() {
      return invokeToolUnwrap<ClipboardSnapshot>(client, EXT, "get_clipboard");
    },
    async setText(text) {
      await invokeToolUnwrap(client, EXT, "set_clipboard", { kind: "text", text });
    },
    async setImage(imagePath) {
      await invokeToolUnwrap(client, EXT, "set_clipboard", { kind: "image", imagePath });
    },
    async setFiles(files) {
      await invokeToolUnwrap(client, EXT, "set_clipboard", { kind: "files", files });
    },
    async listHistory(limit?) {
      const input: Record<string, unknown> = {};
      if (limit !== undefined) input.limit = limit;
      return invokeToolUnwrap<ClipboardEntry[]>(client, EXT, "list_clipboard_history", input);
    },
    async clearHistory() {
      await invokeToolUnwrap(client, EXT, "clear_clipboard_history");
    },
  };
}
