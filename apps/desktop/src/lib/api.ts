import { createApiClient, type Client } from "@cocommand/api-client";

let _client: Client | null = null;

/**
 * Initialise the shared API client with the server address.
 * Call this once when the server info becomes available.
 */
export function initApiClient(addr: string): Client {
  _client = createApiClient(addr);
  return _client;
}

/**
 * Return the shared API client. Throws if not yet initialised.
 */
export function getApiClient(): Client {
  if (!_client) {
    throw new Error("API client not initialised – call initApiClient first");
  }
  return _client;
}

/**
 * Reset the client (e.g. on server reconnect).
 */
export function resetApiClient(): void {
  _client = null;
}
