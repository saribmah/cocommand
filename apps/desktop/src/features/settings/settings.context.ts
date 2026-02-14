import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { SettingsState, SettingsStore } from "./settings.store";

export const SettingsContext = createContext<SettingsStore | null>(null);

export function useSettingsContext<T>(selector: (state: SettingsState) => T): T {
  const store = useContext(SettingsContext);
  if (!store) {
    throw new Error("Missing SettingsContext.Provider in the tree");
  }
  return useStore(store, selector);
}
