import type { Config, ClientOptions } from "@hey-api/client-fetch";

/**
 * Default client configuration for the Hey API generated client.
 * The base URL is set at runtime via `client.setConfig({ baseUrl })`.
 */
export const createClientConfig = <T extends ClientOptions>(
  override?: Config<T>,
): Config<T> => {
  return {
    ...override,
    baseUrl: override?.baseUrl ?? "",
  } as Config<T>;
};
