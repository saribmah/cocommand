import { createClient, type Client } from "@hey-api/client-fetch";

export type { Client };

/**
 * Create a typed API client for the cocommand server.
 * Returns a hey-api client instance that can be passed to SDK functions
 * via the `client` option.
 *
 * @param baseUrl - Server address, e.g. "http://127.0.0.1:4840"
 */
export function createApiClient(baseUrl: string): Client {
  const prefix = baseUrl.startsWith("http") ? baseUrl : `http://${baseUrl}`;
  return createClient({ baseUrl: prefix });
}
