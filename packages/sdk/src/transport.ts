import type { InvokeResponse } from "./types";

export interface Transport {
  apiGet<T>(path: string): Promise<T>;
  apiPost<T>(path: string, body?: unknown): Promise<T>;
  apiDelete<T>(path: string): Promise<T>;
  invokeTool<T>(
    extensionId: string,
    toolId: string,
    input?: Record<string, unknown>,
  ): Promise<T>;
}

function normalizeUrl(url: string): string {
  return url.startsWith("http") ? url : `http://${url}`;
}

export function createTransport(baseUrl: string): Transport {
  const base = normalizeUrl(baseUrl);

  return {
    async apiGet<T>(path: string): Promise<T> {
      const url = `${base}${path}`;
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`GET ${path} failed (${response.status})`);
      }
      return response.json() as Promise<T>;
    },

    async apiPost<T>(path: string, body?: unknown): Promise<T> {
      const url = `${base}${path}`;
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: body !== undefined ? JSON.stringify(body) : undefined,
      });
      if (!response.ok) {
        throw new Error(`POST ${path} failed (${response.status})`);
      }
      return response.json() as Promise<T>;
    },

    async apiDelete<T>(path: string): Promise<T> {
      const url = `${base}${path}`;
      const response = await fetch(url, { method: "DELETE" });
      if (!response.ok) {
        throw new Error(`DELETE ${path} failed (${response.status})`);
      }
      return response.json() as Promise<T>;
    },

    async invokeTool<T>(
      extensionId: string,
      toolId: string,
      input: Record<string, unknown> = {},
    ): Promise<T> {
      const url = `${base}/extension/${extensionId}/invoke/${toolId}`;
      const response = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
      const result = (await response.json()) as InvokeResponse<T>;
      if (!result.ok || !response.ok) {
        throw new Error(
          result.error?.message ?? `Server error (${response.status})`,
        );
      }
      return result.data as T;
    },
  };
}
