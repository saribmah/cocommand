/**
 * A single clipboard history entry.
 * Matches backend ClipboardHistoryEntry (serde fields are snake_case).
 */
export interface ClipboardEntry {
  id: string;
  created_at: string;
  kind: "text" | "image" | "files";
  text?: string;
  image_path?: string;
  image_format?: string;
  files?: string[];
  source?: string;
}
