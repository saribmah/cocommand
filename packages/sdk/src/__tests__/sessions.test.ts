import { afterEach, describe, expect, it } from "bun:test";
import type { SessionCommandInputPart } from "@cocommand/api";
import { createApiClient } from "../client";
import { SdkError } from "../errors";
import { createSessionsApi } from "../sessions";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

function createSseResponse(frames: string[]): Response {
  return new Response(
    new ReadableStream<Uint8Array>({
      start(controller) {
        for (const frame of frames) {
          controller.enqueue(new TextEncoder().encode(frame));
        }
        controller.close();
      },
    }),
  );
}

const commandParts: SessionCommandInputPart[] = [{ type: "text", text: "Hello" }];

describe("sessions.commandStream", () => {
  it("returns final response from done event", async () => {
    globalThis.fetch = (() =>
      Promise.resolve(
        createSseResponse([
          "event: part.updated\ndata: {\"part_id\":\"p1\",\"part\":{\"type\":\"text\",\"id\":\"p1\",\"messageId\":\"m1\",\"sessionId\":\"s1\",\"text\":\"hello\"}}\n\n",
          "event: context\ndata: {\"context\":{\"workspace_id\":\"w1\",\"session_id\":\"s1\",\"started_at\":1,\"ended_at\":null}}\n\n",
          "event: done\ndata: {\"context\":{\"workspace_id\":\"w1\",\"session_id\":\"s1\",\"started_at\":1,\"ended_at\":null},\"reply_parts\":[{\"type\":\"text\",\"id\":\"r1\",\"messageId\":\"m2\",\"sessionId\":\"s1\",\"text\":\"ok\"}]}\n\n",
        ]),
      )) as typeof fetch;

    const sessions = createSessionsApi(createApiClient("http://localhost:8080"));
    const result = await sessions.command(commandParts);

    expect(result.context.session_id).toBe("s1");
    expect(result.reply_parts.length).toBe(1);
  });

  it("throws typed sse_error when error event is emitted", async () => {
    globalThis.fetch = (() =>
      Promise.resolve(
        createSseResponse([
          "event: error\ndata: {\"error\":{\"code\":\"bad_request\",\"message\":\"stream failed\"}}\n\n",
        ]),
      )) as typeof fetch;

    const sessions = createSessionsApi(createApiClient("http://localhost:8080"));
    const consume = async () => {
      for await (const _ of sessions.commandStream(commandParts)) {
        // noop
      }
    };

    await expect(consume()).rejects.toBeInstanceOf(SdkError);

    try {
      await consume();
    } catch (error) {
      expect((error as SdkError).code).toBe("sse_error");
      expect((error as SdkError).message).toContain("stream failed");
    }
  });

  it("surfaces cancellation as typed aborted error", async () => {
    globalThis.fetch = ((_: string, init?: RequestInit) => {
      const signal = init?.signal;
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

    const consume = async () => {
      for await (const _ of sessions.commandStream(commandParts, { signal: controller.signal })) {
        // noop
      }
    };

    await expect(consume()).rejects.toBeInstanceOf(SdkError);

    try {
      await consume();
    } catch (error) {
      expect((error as SdkError).code).toBe("aborted");
    }
  });
});
