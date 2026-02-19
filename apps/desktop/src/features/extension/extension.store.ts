import { create } from "zustand";
import type { StoreApi } from "zustand";
import type { Sdk } from "@cocommand/sdk";
import { getRegisteredStoreFactories } from "./extension-stores";
import { loadDynamicExtensionViews } from "./extension-loader";
import type { ExtensionInfo, ExtensionInvokeFn } from "./extension.types";

export interface ExtensionState {
  extensions: ExtensionInfo[];
  isLoaded: boolean;
  error: string | null;
  dynamicViewsLoaded: boolean;
  viewLoadVersion: number;
  fetchExtensions: () => Promise<void>;
  openExtension: (id: string) => Promise<void>;
  getExtensions: () => ExtensionInfo[];
  loadDynamicViews: () => Promise<void>;
  invoke: ExtensionInvokeFn;
  stores: Record<string, StoreApi<unknown>>;
}

export type ExtensionStore = ReturnType<typeof createExtensionStore>;

export const createExtensionStore = (sdk: Sdk) => {
  const invoke: ExtensionInvokeFn = <T = unknown>(
    extensionId: string,
    toolId: string,
    input?: Record<string, unknown>,
    options?: { signal?: AbortSignal },
  ): Promise<T> => {
    return sdk.tools.invoke<T>(extensionId, toolId, input ?? {}, { signal: options?.signal });
  };

  const stores: Record<string, StoreApi<unknown>> = {};
  for (const [id, factory] of getRegisteredStoreFactories()) {
    stores[id] = factory(invoke);
  }

  return create<ExtensionState>()((set, get) => ({
    extensions: [],
    isLoaded: false,
    error: null,
    dynamicViewsLoaded: false,
    viewLoadVersion: 0,
    invoke,
    stores,
    loadDynamicViews: async () => {
      const { extensions, viewLoadVersion } = get();
      try {
        await loadDynamicExtensionViews(sdk, extensions);
      } catch (err) {
        console.warn("Failed to load dynamic extension views:", err);
      }
      const newStores: Record<string, StoreApi<unknown>> = { ...get().stores };
      for (const [id, factory] of getRegisteredStoreFactories()) {
        if (!newStores[id]) {
          newStores[id] = factory(invoke);
        }
      }
      set({
        stores: newStores,
        dynamicViewsLoaded: true,
        viewLoadVersion: viewLoadVersion + 1,
      });
    },
    fetchExtensions: async () => {
      try {
        const extensions = await sdk.extensions.list();
        set({ extensions, isLoaded: true, error: null });

        const hasDynamicViews = extensions.some(
          (ext) => ext.view && ext.kind === "custom",
        );
        if (hasDynamicViews) {
          get().loadDynamicViews();
        }
      } catch (error) {
        set({ extensions: [], isLoaded: false, error: String(error) });
      }
    },
    openExtension: async (id) => {
      const ext = get().extensions.find((e) => e.id === id);
      if (ext && ext.status !== "ready") {
        throw new Error(
          ext.status === "building"
            ? `Extension "${ext.name}" is still initializing`
            : ext.status === "disabled"
              ? `Extension "${ext.name}" is disabled`
              : `Extension "${ext.name}" is not available (${ext.status})`,
        );
      }

      await sdk.extensions.open(id);
    },
    getExtensions: () => get().extensions,
  }));
};
