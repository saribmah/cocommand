import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { ApplicationState, ApplicationStore } from "./application.store";

export const ApplicationContext = createContext<ApplicationStore | null>(null);

export function useApplicationContext<T>(
  selector: (state: ApplicationState) => T
): T {
  const store = useContext(ApplicationContext);
  if (!store) {
    throw new Error("Missing ApplicationContext.Provider in the tree");
  }
  return useStore(store, selector);
}
