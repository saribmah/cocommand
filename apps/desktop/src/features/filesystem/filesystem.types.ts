export interface SearchRequest {
  query: string;
  root?: string;
  ignorePaths?: string[];
  kind?: "all" | "file" | "directory";
  includeHidden?: boolean;
  caseSensitive?: boolean;
  maxResults?: number;
  maxDepth?: number;
}

export interface SearchEntry {
  path: string;
  name: string;
  type: "file" | "directory" | "symlink" | "other";
  size: number | null;
  modifiedAt: number | null;
}

export interface SearchResponse {
  query: string;
  root: string;
  results: SearchEntry[];
  count: number;
  truncated: boolean;
  scanned: number;
  errors: number;
  indexState: string;
  indexScannedFiles: number;
  indexScannedDirs: number;
  indexStartedAt: number | null;
  indexLastUpdateAt: number | null;
  indexFinishedAt: number | null;
  highlightTerms: string[];
}

export interface IndexStatusRequest {
  root?: string;
  ignorePaths?: string[];
}

export interface IndexStatusResponse {
  state: string;
  root: string;
  ignoredPaths: string[];
  indexedEntries: number;
  scannedFiles: number;
  scannedDirs: number;
  startedAt: number | null;
  lastUpdateAt: number | null;
  finishedAt: number | null;
  errors: number;
  watcherEnabled: boolean;
  cachePath: string;
  rescanCount: number;
  lastError?: string;
}

export type IndexState = "idle" | "building" | "updating" | "ready" | "error";

export function parseIndexState(state: string): IndexState {
  switch (state) {
    case "idle":
    case "building":
    case "updating":
    case "ready":
    case "error":
      return state;
    default:
      return "idle";
  }
}
