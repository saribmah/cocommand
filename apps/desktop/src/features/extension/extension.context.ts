import { createContext, useContext } from "react";
import { useStore } from "zustand";
import type { StoreApi } from "zustand";
import type { ExtensionState, ExtensionStore } from "./extension.store";

export const ExtensionContext = createContext<ExtensionStore | null>(null);

export function useExtensionContext<T>(selector: (state: ExtensionState) => T): T {
  const store = useContext(ExtensionContext);
  if (!store) {
    throw new Error("Missing ExtensionContext.Provider in the tree");
  }
  return useStore(store, selector);
}

export function useExtensionStore<S>(extensionId: string): StoreApi<S> {
  const stores = useExtensionContext((s) => s.stores);
  const store = stores[extensionId];
  if (!store) throw new Error(`No store registered for extension "${extensionId}"`);
  return store as StoreApi<S>;
}
