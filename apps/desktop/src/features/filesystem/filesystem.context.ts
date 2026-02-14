import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { FileSystemState, FileSystemStore } from "./filesystem.store";

export const FileSystemContext = createContext<FileSystemStore | null>(null);

export function useFileSystemContext<T>(selector: (state: FileSystemState) => T): T {
  const store = useContext(FileSystemContext);
  if (!store) {
    throw new Error("Missing FileSystemContext.Provider in the tree");
  }
  return useStore(store, selector);
}
