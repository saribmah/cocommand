import { afterEach, describe, expect, it } from "bun:test";
import type { SessionCommandInputPart } from "@cocommand/api";
import { createApiClient } from "../client";
import { SdkError } from "../errors";
import { createSessionsApi } from "../sessions";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

const commandParts: SessionCommandInputPart[] = [{ type: "text", text: "Hello" }];

describe("sessions api", () => {
  it("loads session history as messages", async () => {
    globalThis.fetch = ((input: RequestInfo | URL) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof Request
            ? input.url
            : String(input);

      if (url.endsWith("/session/command")) {
        return Promise.resolve(
          new Response(
            JSON.stringify([
              {
                info: {
                  id: "m1",
                  sessionId: "s1",
                  role: "user",
                  createdAt: "2025-01-01T00:00:00Z",
                },
                parts: [
                  {
                    type: "text",
                    id: "u1",
                    messageId: "m1",
                    sessionId: "s1",
                    text: "Hello",
                  },
                ],
              },
              {
                info: {
                  id: "m2",
                  sessionId: "s1",
                  role: "assistant",
                  createdAt: "2025-01-01T00:00:01Z",
                },
                parts: [
                  {
                    type: "text",
                    id: "r1",
                    messageId: "m2",
                    sessionId: "s1",
                    text: "Hi there",
                  },
                ],
              },
            ]),
            {
              status: 200,
              headers: { "Content-Type": "application/json" },
            },
          ),
        );
      }

      return Promise.resolve(new Response("Not Found", { status: 404 }));
    }) as typeof fetch;

    const sessions = createSessionsApi(createApiClient("http://localhost:8080"));
    const history = await sessions.history();

    expect(history.length).toBe(2);
    expect(history[0]?.info.role).toBe("user");
    expect(history[1]?.info.role).toBe("assistant");
  });

  it("returns enqueue response for command", async () => {
    globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
      const url =
        typeof input === "string"
          ? input
          : input instanceof Request
            ? input.url
            : String(input);
      const method =
        init?.method ??
        (input instanceof Request ? input.method : undefined);

      if (url.endsWith("/sessions/command")) {
        expect(method).toBe("POST");
        return Promise.resolve(
          new Response(
            JSON.stringify({
              context: {
                workspace_id: "w1",
                session_id: "s1",
                started_at: 1,
                ended_at: null,
              },
              run_id: "run-1",
              accepted_at: 123,
            }),
            {
              status: 200,
              headers: { "Content-Type": "application/json" },
            },
          ),
        );
      }

      return Promise.resolve(new Response("Not Found", { status: 404 }));
    }) as typeof fetch;

    const sessions = createSessionsApi(createApiClient("http://localhost:8080"));
    const result = await sessions.command(commandParts);

    expect(result.context.session_id).toBe("s1");
    expect(result.run_id).toBe("run-1");
    expect(result.accepted_at).toBe(123);
  });

  it("surfaces cancellation as typed aborted error", async () => {
    globalThis.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
      const signal =
        init?.signal ??
        (input instanceof Request ? input.signal : undefined);
      return new Promise<Response>((_, reject) => {
        if (signal instanceof AbortSignal && signal.aborted) {
          reject(new DOMException("Aborted", "AbortError"));
          return;
        }

        if (signal instanceof AbortSignal) {
          signal.addEventListener(
            "abort",
            () => reject(new DOMException("Aborted", "AbortError")),
            { once: true },
          );
        }
      });
    }) as typeof fetch;

    const sessions = createSessionsApi(createApiClient("http://localhost:8080"));
    const controller = new AbortController();
    controller.abort();

    await expect(
      sessions.command(commandParts, { signal: controller.signal }),
    ).rejects.toBeInstanceOf(SdkError);

    try {
      await sessions.command(commandParts, { signal: controller.signal });
    } catch (error) {
      expect((error as SdkError).code).toBe("aborted");
    }
  });
});
