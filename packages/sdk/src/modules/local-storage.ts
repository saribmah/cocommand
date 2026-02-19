import type { Client } from "../client";

export interface LocalStorageApi {
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T): Promise<void>;
  delete(key: string): Promise<void>;
  keys(): Promise<string[]>;
}

export function createLocalStorage(_client: Client, _extensionId: string): LocalStorageApi {
  return {
    async get<T>(_key: string): Promise<T | null> {
      throw new Error(
        "@cocommand/sdk: LocalStorage.get() is not yet implemented. " +
        "Requires GET /extension/:id/storage/:key Rust endpoint.",
      );
    },
    async set<T>(_key: string, _value: T): Promise<void> {
      throw new Error(
        "@cocommand/sdk: LocalStorage.set() is not yet implemented. " +
        "Requires PUT /extension/:id/storage/:key Rust endpoint.",
      );
    },
    async delete(_key: string): Promise<void> {
      throw new Error(
        "@cocommand/sdk: LocalStorage.delete() is not yet implemented. " +
        "Requires DELETE /extension/:id/storage/:key Rust endpoint.",
      );
    },
    async keys(): Promise<string[]> {
      throw new Error(
        "@cocommand/sdk: LocalStorage.keys() is not yet implemented. " +
        "Requires GET /extension/:id/storage Rust endpoint.",
      );
    },
  };
}
