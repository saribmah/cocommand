export { createApiClient, type Client } from "@cocommand/api";
import { invokeTool } from "@cocommand/api";
import type { Client } from "@cocommand/api";
import { SdkError, normalizeUnknownError } from "./errors";

export interface InvokeResponse<T = unknown> {
  ok: boolean;
  data?: T;
  error?: { code?: string; message?: string };
}

export interface RequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}

export function unwrapInvokeEnvelope<T>(
  extensionId: string,
  toolId: string,
  envelope: InvokeResponse<T> | undefined,
): T {
  if (!envelope) {
    throw new SdkError({
      code: "invalid_response",
      message: `Missing invoke envelope for ${extensionId}.${toolId}`,
      source: `${extensionId}.${toolId}`,
    });
  }

  if (!envelope.ok) {
    throw new SdkError({
      code: "tool_error",
      message:
        envelope.error?.message ??
        `Tool returned ok:false for ${extensionId}.${toolId}`,
      source: `${extensionId}.${toolId}`,
      details: envelope.error,
    });
  }

  return envelope.data as T;
}

interface AbortBinding {
  signal?: AbortSignal;
  cleanup: () => void;
  didTimeout: () => boolean;
}

function bindAbortSignal(options?: RequestOptions): AbortBinding {
  const timeoutMs = options?.timeoutMs;
  const externalSignal = options?.signal;

  if (!externalSignal && timeoutMs === undefined) {
    return {
      signal: undefined,
      cleanup: () => {},
      didTimeout: () => false,
    };
  }

  const controller = new AbortController();
  let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
  let timedOut = false;

  const onAbort = () => {
    controller.abort((externalSignal as AbortSignal & { reason?: unknown }).reason);
  };

  if (externalSignal) {
    if (externalSignal.aborted) {
      controller.abort((externalSignal as AbortSignal & { reason?: unknown }).reason);
    } else {
      externalSignal.addEventListener("abort", onAbort, { once: true });
    }
  }

  if (timeoutMs !== undefined) {
    timeoutHandle = setTimeout(() => {
      timedOut = true;
      controller.abort(new Error(`Timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  }

  return {
    signal: controller.signal,
    cleanup: () => {
      if (timeoutHandle !== undefined) {
        clearTimeout(timeoutHandle);
      }
      if (externalSignal) {
        externalSignal.removeEventListener("abort", onAbort);
      }
    },
    didTimeout: () => timedOut,
  };
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

export function resolveClientBaseUrl(client: Client): string {
  const baseUrl = client.getConfig().baseUrl;
  if (!baseUrl) {
    throw new SdkError({
      code: "invalid_response",
      message: "Client baseUrl is not configured",
      source: "client",
    });
  }
  return String(baseUrl).replace(/\/$/, "");
}

export async function invokeToolUnwrap<T>(
  client: Client,
  extensionId: string,
  toolId: string,
  input: Record<string, unknown> = {},
  options?: RequestOptions,
): Promise<T> {
  const abort = bindAbortSignal(options);

  try {
    const { data, error, response } = await invokeTool({
      client,
      path: { extension_id: extensionId, tool_id: toolId },
      body: input,
      signal: abort.signal,
    });

    if (error || !response.ok) {
      const parsed = readApiError(error);
      throw new SdkError({
        code: "http_error",
        message:
          parsed?.message ??
          `HTTP ${response.status} while invoking ${extensionId}.${toolId}`,
        status: response.status,
        source: `${extensionId}.${toolId}`,
        details: parsed ?? error,
      });
    }

    return unwrapInvokeEnvelope<T>(
      extensionId,
      toolId,
      data as InvokeResponse<T> | undefined,
    );
  } catch (error) {
    if (error instanceof SdkError) {
      throw error;
    }

    const aborted = options?.signal?.aborted === true;
    if (aborted || abort.didTimeout()) {
      throw new SdkError({
        code: abort.didTimeout() ? "timeout" : "aborted",
        message: abort.didTimeout()
          ? `Invoke timed out for ${extensionId}.${toolId}`
          : `Invoke aborted for ${extensionId}.${toolId}`,
        source: `${extensionId}.${toolId}`,
        details: error,
      });
    }

    throw new SdkError({
      code: "api_error",
      message: normalizeUnknownError(error),
      source: `${extensionId}.${toolId}`,
      details: error,
    });
  } finally {
    abort.cleanup();
  }
}

export async function invokeExtensionTool<T>(
  client: Client,
  extensionId: string,
  toolId: string,
  input: Record<string, unknown> = {},
  options?: RequestOptions,
): Promise<T> {
  return invokeToolUnwrap<T>(client, extensionId, toolId, input, options);
}
