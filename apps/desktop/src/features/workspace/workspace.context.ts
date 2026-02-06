import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { WorkspaceState, WorkspaceStore } from "./workspace.store";

export const WorkspaceContext = createContext<WorkspaceStore | null>(null);

export function useWorkspaceContext<T>(selector: (state: WorkspaceState) => T): T {
  const store = useContext(WorkspaceContext);
  if (!store) {
    throw new Error("Missing WorkspaceContext.Provider in the tree");
  }
  return useStore(store, selector);
}
