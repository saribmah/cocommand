import { create } from "zustand";
import type { StoreApi } from "zustand";
import { invokeExtensionTool } from "../../lib/extension-client";
import { getRegisteredStoreFactories } from "./extension-stores";
import type { ExtensionInfo, ExtensionInvokeFn } from "./extension.types";

export interface ExtensionState {
  extensions: ExtensionInfo[];
  isLoaded: boolean;
  error: string | null;
  fetchExtensions: () => Promise<void>;
  openExtension: (id: string) => Promise<void>;
  getExtensions: () => ExtensionInfo[];
  invoke: ExtensionInvokeFn;
  stores: Record<string, StoreApi<unknown>>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export type ExtensionStore = ReturnType<typeof createExtensionStore>;

export const createExtensionStore = (getAddr: () => string | null) => {
  const invoke: ExtensionInvokeFn = <T = unknown>(
    extensionId: string,
    toolId: string,
    input?: Record<string, unknown>,
    options?: { signal?: AbortSignal },
  ): Promise<T> => {
    const addr = getAddr();
    if (!addr) throw new Error("Server unavailable");
    return invokeExtensionTool<T>(addr, extensionId, toolId, input ?? {}, options);
  };

  const stores: Record<string, StoreApi<unknown>> = {};
  for (const [id, factory] of getRegisteredStoreFactories()) {
    stores[id] = factory(invoke);
  }

  return create<ExtensionState>()((set, get) => ({
    extensions: [],
    isLoaded: false,
    error: null,
    invoke,
    stores,
    fetchExtensions: async () => {
      const addr = getAddr();
      if (!addr) {
        set({ extensions: [], isLoaded: false, error: null });
        return;
      }

      const url = buildServerUrl(addr, "/workspace/extensions");
      try {
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`Server error (${response.status})`);
        }
        const data = (await response.json()) as ExtensionInfo[];
        set({ extensions: data, isLoaded: true, error: null });
      } catch (error) {
        set({ extensions: [], isLoaded: false, error: String(error) });
      }
    },
    openExtension: async (id) => {
      const addr = getAddr();
      if (!addr) {
        throw new Error("Server unavailable");
      }

      const url = buildServerUrl(addr, "/workspace/extensions/open");
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id }),
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `Server error (${response.status})`);
      }
    },
    getExtensions: () => get().extensions,
  }));
};
