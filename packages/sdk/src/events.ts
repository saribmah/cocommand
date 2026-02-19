import type { Client, CoreEvent } from "@cocommand/api";
import { fetchSseGet } from "./request";
import { readSse } from "./sse";

export interface EventsApi {
  stream(options?: { signal?: AbortSignal; timeoutMs?: number }): AsyncGenerator<CoreEvent>;
}

export function createEventsApi(client: Client): EventsApi {
  return {
    async *stream(options) {
      const response = await fetchSseGet(client, "/events", options);

      for await (const event of readSse(response)) {
        const payload = event.data as CoreEvent;
        yield payload;
      }
    },
  };
}
