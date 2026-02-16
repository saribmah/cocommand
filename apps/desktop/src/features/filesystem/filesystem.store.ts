import { create } from "zustand";
import type { ServerInfo } from "../../lib/ipc";
import type {
  IndexStatusRequest,
  IndexStatusResponse,
  IndexState,
  SearchRequest,
  SearchResponse,
} from "./filesystem.types";
import { parseIndexState } from "./filesystem.types";

export interface FileSystemState {
  // Index status
  indexStatus: IndexStatusResponse | null;
  indexState: IndexState;
  isLoading: boolean;
  error: string | null;
  fetchIndexStatus: (request?: IndexStatusRequest) => Promise<void>;

  // Search
  searchResults: SearchResponse | null;
  searchQuery: string;
  isSearching: boolean;
  searchError: string | null;
  search: (request: SearchRequest) => Promise<void>;
  clearSearch: () => void;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export type FileSystemStore = ReturnType<typeof createFileSystemStore>;

export const createFileSystemStore = (getServer: () => ServerInfo | null) => {
  // Track the current search version for cancellation
  let currentSearchVersion: number | null = null;
  // Monotonic local version counter
  let nextSearchVersion = 0;
  // Abort in-flight HTTP search when a newer query starts.
  let activeSearchController: AbortController | null = null;

  return create<FileSystemState>()((set) => ({
    // Index status state
    indexStatus: null,
    indexState: "idle",
    isLoading: false,
    error: null,

    // Search state
    searchResults: null,
    searchQuery: "",
    isSearching: false,
    searchError: null,

    fetchIndexStatus: async (request?: IndexStatusRequest) => {
      const server = getServer();
      if (!server || !server.addr) {
        set({ indexStatus: null, indexState: "idle", isLoading: false, error: null });
        return;
      }
      set({ isLoading: true, error: null });
      const url = buildServerUrl(server.addr, "/extension/filesystem/status");
      try {
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(request ?? {}),
        });
        if (!response.ok) {
          throw new Error(`Server error (${response.status})`);
        }
        const data = (await response.json()) as IndexStatusResponse;
        set({
          indexStatus: data,
          indexState: parseIndexState(data.state),
          isLoading: false,
          error: null,
        });
      } catch (error) {
        set({
          indexStatus: null,
          indexState: "error",
          isLoading: false,
          error: String(error),
        });
      }
    },

    search: async (request: SearchRequest) => {
      const server = getServer();
      if (!server || !server.addr) {
        set({ searchResults: null, isSearching: false, searchError: "Server unavailable" });
        return;
      }

      const version = ++nextSearchVersion;
      currentSearchVersion = version;
      if (activeSearchController) {
        activeSearchController.abort();
      }
      const controller = new AbortController();
      activeSearchController = controller;

      set({ searchQuery: request.query, isSearching: true, searchError: null });

      try {
        // Perform the search with the version
        const searchUrl = buildServerUrl(server.addr, "/extension/filesystem/search");
        const response = await fetch(searchUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ ...request, searchVersion: version }),
          signal: controller.signal,
        });

        // 204 No Content means the search was cancelled by a newer search
        if (response.status === 204) {
          // Only clear isSearching if this was the latest version
          if (currentSearchVersion === version) {
            set({ isSearching: false });
          }
          return;
        }

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Server error (${response.status})`);
        }

        const data = (await response.json()) as SearchResponse;

        // Only update state if this is still the active search version
        if (currentSearchVersion === version) {
          set({
            searchResults: data,
            isSearching: false,
            searchError: null,
          });
        }
      } catch (error) {
        if (currentSearchVersion !== version) {
          return;
        }
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        set({
          searchResults: null,
          isSearching: false,
          searchError: String(error),
        });
      } finally {
        if (activeSearchController === controller) {
          activeSearchController = null;
        }
      }
    },

    clearSearch: () => {
      if (activeSearchController) {
        activeSearchController.abort();
        activeSearchController = null;
      }
      currentSearchVersion = null;
      set({
        searchResults: null,
        searchQuery: "",
        isSearching: false,
        searchError: null,
      });
    },
  }));
};
