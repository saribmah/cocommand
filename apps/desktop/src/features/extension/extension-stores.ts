import type { StoreApi } from "zustand";
import type { ExtensionInvokeFn } from "./extension.types";

export type ExtensionStoreFactory = (invoke: ExtensionInvokeFn) => StoreApi<unknown>;

export type ExtensionStoreSource = "builtin" | "dynamic";

interface RegisteredExtensionStoreFactory {
  factory: ExtensionStoreFactory;
  source: ExtensionStoreSource;
}

const factories = new Map<string, RegisteredExtensionStoreFactory>();

export function registerExtensionStore(
  extensionId: string,
  factory: ExtensionStoreFactory,
  options?: { source?: ExtensionStoreSource },
): void {
  factories.set(extensionId, {
    factory,
    source: options?.source ?? "dynamic",
  });
}

export function getRegisteredStoreFactories(): ReadonlyMap<string, ExtensionStoreFactory> {
  const byId = new Map<string, ExtensionStoreFactory>();
  for (const [extensionId, entry] of factories.entries()) {
    byId.set(extensionId, entry.factory);
  }
  return byId;
}

export function resetDynamicExtensionStoreFactories(): void {
  for (const [extensionId, entry] of factories.entries()) {
    if (entry.source === "dynamic") {
      factories.delete(extensionId);
    }
  }
}
