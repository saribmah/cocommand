import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { SessionState, SessionStore } from "./session.store";

export const SessionContext = createContext<SessionStore | null>(null);

export function useSessionContext<T>(selector: (state: SessionState) => T): T {
  const store = useContext(SessionContext);
  if (!store) {
    throw new Error("Missing SessionContext.Provider in the tree");
  }
  return useStore(store, selector);
}
