import type { StoreApi } from "zustand";
import type { ExtensionInvokeFn } from "./extension.types";

export type ExtensionStoreFactory = (invoke: ExtensionInvokeFn) => StoreApi<unknown>;

const factories = new Map<string, ExtensionStoreFactory>();

export function registerExtensionStore(extensionId: string, factory: ExtensionStoreFactory): void {
  factories.set(extensionId, factory);
}

export function getRegisteredStoreFactories(): ReadonlyMap<string, ExtensionStoreFactory> {
  return factories;
}
