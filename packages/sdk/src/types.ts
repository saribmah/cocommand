import type {
  FilePartSourceText,
  ApplicationInfo,
} from "@cocommand/api";

/** Alias for API's FilePartSourceText — identical shape. */
export type TextSource = FilePartSourceText;

/** Alias for API's ApplicationInfo. */
export type Application = ApplicationInfo;

export interface FilePartInput {
  type: "file";
  path: string;
  name: string;
  entryType?: "file" | "directory" | "symlink" | "other" | null;
  source?: TextSource | null;
}

export interface ExtensionPartInput {
  type: "extension";
  extensionId: string;
  name: string;
  kind?: string | null;
  source?: TextSource | null;
}

export interface WorkspaceConfig {
  [key: string]: unknown;
}

export interface ClipboardSnapshot {
  text?: string | null;
  html?: string | null;
  image?: string | null;
  files?: string[] | null;
}

export interface ClipboardEntry {
  id: string;
  text?: string | null;
  timestamp: number;
}

export interface InvokeResponse<T = unknown> {
  ok: boolean;
  data?: T;
  error?: { code: string; message: string };
}
