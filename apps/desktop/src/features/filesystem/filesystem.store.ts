import { create } from "zustand";
import type { ExtensionInvokeFn } from "../extension/extension.types";
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

export type FileSystemStore = ReturnType<typeof createFileSystemStore>;

export const createFileSystemStore = (invoke: ExtensionInvokeFn) => {
  // Monotonic local version counter for stale-result detection
  let currentSearchVersion = 0;
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
      set({ isLoading: true, error: null });
      try {
        const data = await invoke<IndexStatusResponse>(
          "filesystem",
          "index_status",
          (request as Record<string, unknown>) ?? {},
        );
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
      const version = ++currentSearchVersion;
      if (activeSearchController) {
        activeSearchController.abort();
      }
      const controller = new AbortController();
      activeSearchController = controller;

      set({ searchQuery: request.query, isSearching: true, searchError: null });

      try {
        const data = await invoke<SearchResponse>(
          "filesystem",
          "search",
          request as unknown as Record<string, unknown>,
          { signal: controller.signal },
        );

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
      currentSearchVersion = 0;
      set({
        searchResults: null,
        searchQuery: "",
        isSearching: false,
        searchError: null,
      });
    },
  }));
};
