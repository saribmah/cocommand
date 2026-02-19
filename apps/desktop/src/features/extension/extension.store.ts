import { create } from "zustand";
import type { StoreApi } from "zustand";
import { createApiClient } from "@cocommand/api";
import { listExtensions, openExtension } from "@cocommand/api";
import { invokeExtensionTool } from "../../lib/extension-client";
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
  loadDynamicViews: (serverAddr: string) => Promise<void>;
  invoke: ExtensionInvokeFn;
  stores: Record<string, StoreApi<unknown>>;
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
    dynamicViewsLoaded: false,
    viewLoadVersion: 0,
    invoke,
    stores,
    loadDynamicViews: async (serverAddr: string) => {
      const { extensions, viewLoadVersion } = get();
      try {
        await loadDynamicExtensionViews(extensions, serverAddr);
      } catch (err) {
        console.warn("Failed to load dynamic extension views:", err);
      }
      // Create stores for any newly registered factories
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
      const addr = getAddr();
      if (!addr) {
        set({ extensions: [], isLoaded: false, error: null });
        return;
      }

      try {
        const client = createApiClient(addr);
        const { data, error: fetchError } = await listExtensions({ client });
        if (fetchError) {
          throw new Error("Server error");
        }
        const extensions = data as ExtensionInfo[];
        set({ extensions, isLoaded: true, error: null });

        // Fire dynamic view loading if any custom extensions have views
        const hasDynamicViews = extensions.some(
          (ext) => ext.view && ext.kind === "custom"
        );
        if (hasDynamicViews) {
          get().loadDynamicViews(addr);
        }
      } catch (error) {
        set({ extensions: [], isLoaded: false, error: String(error) });
      }
    },
    openExtension: async (id) => {
      const addr = getAddr();
      if (!addr) {
        throw new Error("Server unavailable");
      }

      // Check if extension is ready before opening
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

      const client = createApiClient(addr);
      const { error: fetchError } = await openExtension({
        client,
        body: { id },
      });
      if (fetchError) {
        const msg = fetchError.error?.message ?? "Server error";
        throw new Error(msg);
      }
    },
    getExtensions: () => get().extensions,
  }));
};
