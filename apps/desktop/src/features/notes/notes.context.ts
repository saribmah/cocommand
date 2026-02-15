import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { NotesState, NotesStore } from "./notes.store";

export const NotesContext = createContext<NotesStore | null>(null);

export function useNotesContext<T>(selector: (state: NotesState) => T): T {
  const store = useContext(NotesContext);
  if (!store) {
    throw new Error("Missing NotesContext.Provider in the tree");
  }
  return useStore(store, selector);
}
