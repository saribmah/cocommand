import type { Client } from "./client";
import { resolveClientBaseUrl } from "./client";
import { SdkError, normalizeUnknownError } from "./errors";

export interface ApiResult<TData = unknown, TError = unknown> {
  data?: TData;
  error?: TError;
  response: Response;
}

function readApiError(error: unknown): { code?: string; message?: string } | null {
  if (!error || typeof error !== "object") {
    return null;
  }
  const maybe = error as { error?: { code?: string; message?: string }; message?: string; code?: string };
  if (maybe.error && (maybe.error.code || maybe.error.message)) {
    return maybe.error;
  }
  if (maybe.code || maybe.message) {
    return { code: maybe.code, message: maybe.message };
  }
  return null;
}

export function unwrapApiResponse<TData>(
  operation: string,
  result: ApiResult<TData>,
  options?: { allowNull?: boolean },
): TData {
  if (result.error || !result.response.ok) {
    const parsed = readApiError(result.error);
    throw new SdkError({
      code: "http_error",
      message:
        parsed?.message ?? `HTTP ${result.response.status} for ${operation}`,
      status: result.response.status,
      source: operation,
      details: parsed ?? result.error,
    });
  }

  if (!options?.allowNull && (result.data === undefined || result.data === null)) {
    throw new SdkError({
      code: "invalid_response",
      message: `Missing response payload for ${operation}`,
      source: operation,
    });
  }

  return result.data as TData;
}

export async function fetchSse(
  client: Client,
  path: string,
  payload: unknown,
  options?: { signal?: AbortSignal; timeoutMs?: number },
): Promise<Response> {
  const baseUrl = resolveClientBaseUrl(client);
  const controller = new AbortController();
  let timeoutHandle: ReturnType<typeof setTimeout> | undefined;

  const forwardAbort = () => {
    controller.abort((options?.signal as AbortSignal & { reason?: unknown } | undefined)?.reason);
  };

  if (options?.signal) {
    if (options.signal.aborted) {
      controller.abort((options.signal as AbortSignal & { reason?: unknown }).reason);
    } else {
      options.signal.addEventListener("abort", forwardAbort, { once: true });
    }
  }

  if (options?.timeoutMs !== undefined) {
    timeoutHandle = setTimeout(() => {
      controller.abort(new Error(`Timed out after ${options.timeoutMs}ms`));
    }, options.timeoutMs);
  }

  try {
    const response = await fetch(`${baseUrl}${path}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "text/event-stream",
      },
      body: JSON.stringify(payload),
      signal: controller.signal,
    });

    if (!response.ok) {
      let body: unknown = undefined;
      try {
        body = await response.json();
      } catch {
        try {
          body = await response.text();
        } catch {
          body = undefined;
        }
      }
      const parsed = readApiError(body);
      throw new SdkError({
        code: "http_error",
        message: parsed?.message ?? `HTTP ${response.status} for ${path}`,
        status: response.status,
        source: path,
        details: body,
      });
    }

    return response;
  } catch (error) {
    if (error instanceof SdkError) {
      throw error;
    }
    const wasAborted = options?.signal?.aborted === true;
    const wasTimeout =
      options?.timeoutMs !== undefined &&
      controller.signal.aborted &&
      !wasAborted;
    throw new SdkError({
      code: wasTimeout ? "timeout" : wasAborted ? "aborted" : "sse_error",
      message: wasTimeout
        ? `Timed out after ${options?.timeoutMs}ms`
        : wasAborted
          ? `Request aborted for ${path}`
          : normalizeUnknownError(error),
      source: path,
      details: error,
    });
  } finally {
    if (timeoutHandle !== undefined) {
      clearTimeout(timeoutHandle);
    }
    if (options?.signal) {
      options.signal.removeEventListener("abort", forwardAbort);
    }
  }
}

export async function fetchSseGet(
  client: Client,
  path: string,
  options?: { signal?: AbortSignal; timeoutMs?: number },
): Promise<Response> {
  const baseUrl = resolveClientBaseUrl(client);
  const controller = new AbortController();
  let timeoutHandle: ReturnType<typeof setTimeout> | undefined;

  const forwardAbort = () => {
    controller.abort((options?.signal as AbortSignal & { reason?: unknown } | undefined)?.reason);
  };

  if (options?.signal) {
    if (options.signal.aborted) {
      controller.abort((options.signal as AbortSignal & { reason?: unknown }).reason);
    } else {
      options.signal.addEventListener("abort", forwardAbort, { once: true });
    }
  }

  if (options?.timeoutMs !== undefined) {
    timeoutHandle = setTimeout(() => {
      controller.abort(new Error(`Timed out after ${options.timeoutMs}ms`));
    }, options.timeoutMs);
  }

  try {
    const response = await fetch(`${baseUrl}${path}`, {
      method: "GET",
      headers: {
        Accept: "text/event-stream",
      },
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new SdkError({
        code: "http_error",
        message: `HTTP ${response.status} for ${path}`,
        status: response.status,
        source: path,
      });
    }

    return response;
  } catch (error) {
    if (error instanceof SdkError) {
      throw error;
    }
    const wasAborted = options?.signal?.aborted === true;
    const wasTimeout =
      options?.timeoutMs !== undefined &&
      controller.signal.aborted &&
      !wasAborted;
    throw new SdkError({
      code: wasTimeout ? "timeout" : wasAborted ? "aborted" : "sse_error",
      message: wasTimeout
        ? `Timed out after ${options?.timeoutMs}ms`
        : wasAborted
          ? `Request aborted for ${path}`
          : normalizeUnknownError(error),
      source: path,
      details: error,
    });
  } finally {
    if (timeoutHandle !== undefined) {
      clearTimeout(timeoutHandle);
    }
    if (options?.signal) {
      options.signal.removeEventListener("abort", forwardAbort);
    }
  }
}
