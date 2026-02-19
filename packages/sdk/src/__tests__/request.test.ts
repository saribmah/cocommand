import { afterEach, describe, expect, it } from "bun:test";
import { createApiClient } from "../client";
import { SdkError } from "../errors";
import { fetchSse, unwrapApiResponse } from "../request";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe("unwrapApiResponse", () => {
  it("returns data for successful responses", () => {
    const result = unwrapApiResponse("extensions.list", {
      data: [{ id: "notes" }],
      error: undefined,
      response: new Response(null, { status: 200 }),
    });

    expect(result).toEqual([{ id: "notes" }]);
  });

  it("throws http_error for failed responses", () => {
    expect(() =>
      unwrapApiResponse("extensions.list", {
        data: undefined,
        error: { error: { code: "bad_request", message: "oops" } },
        response: new Response(null, { status: 400 }),
      }),
    ).toThrow(SdkError);
  });
});

describe("fetchSse error normalization", () => {
  it("maps timeout to timeout SdkError", async () => {
    globalThis.fetch = ((_: string, init?: RequestInit) =>
      new Promise<Response>((_, reject) => {
        const signal = init?.signal;
        if (signal instanceof AbortSignal) {
          signal.addEventListener(
            "abort",
            () => {
              reject(new DOMException("Aborted", "AbortError"));
            },
            { once: true },
          );
        }
      })) as typeof fetch;

    const client = createApiClient("http://localhost:8080");
    await expect(fetchSse(client, "/sessions/command", {}, { timeoutMs: 5 })).rejects.toBeInstanceOf(SdkError);

    try {
      await fetchSse(client, "/sessions/command", {}, { timeoutMs: 5 });
    } catch (error) {
      expect((error as SdkError).code).toBe("timeout");
    }
  });

  it("maps external abort to aborted SdkError", async () => {
    globalThis.fetch = ((_: string, init?: RequestInit) => {
      const signal = init?.signal;
      if (signal instanceof AbortSignal && signal.aborted) {
        return Promise.reject(new DOMException("Aborted", "AbortError"));
      }
      return Promise.resolve(new Response(null, { status: 200 }));
    }) as typeof fetch;

    const client = createApiClient("http://localhost:8080");
    const controller = new AbortController();
    controller.abort();

    await expect(
      fetchSse(client, "/sessions/command", {}, { signal: controller.signal }),
    ).rejects.toBeInstanceOf(SdkError);

    try {
      await fetchSse(client, "/sessions/command", {}, { signal: controller.signal });
    } catch (error) {
      expect((error as SdkError).code).toBe("aborted");
    }
  });
});
