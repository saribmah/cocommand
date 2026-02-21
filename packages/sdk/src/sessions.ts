import type {
  ApiSessionContext,
  EnqueueMessageResponse,
  Message,
  SessionCommandInputPart,
  SessionContextData,
  Client,
} from "@cocommand/api";
import { sessionCommand, sessionCommandHistory, sessionContext } from "@cocommand/api";
import { SdkError, normalizeUnknownError } from "./errors";
import { unwrapApiResponse } from "./request";

export interface SessionCommandOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}

export interface SessionsApi {
  command(
    parts: SessionCommandInputPart[],
    options?: SessionCommandOptions,
  ): Promise<EnqueueMessageResponse>;
  history(): Promise<Message[]>;
  context(query?: SessionContextData["query"]): Promise<ApiSessionContext>;
}

export function createSessionsApi(client: Client): SessionsApi {
  return {
    async command(parts, options) {
      const controller = new AbortController();
      let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
      let timedOut = false;

      const forwardAbort = () => {
        controller.abort(
          (options?.signal as (AbortSignal & { reason?: unknown }) | undefined)?.reason,
        );
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
          timedOut = true;
          controller.abort(new Error(`Timed out after ${options.timeoutMs}ms`));
        }, options.timeoutMs);
      }

      try {
        const result = await sessionCommand({
          client,
          body: { parts },
          signal: controller.signal,
        });
        return unwrapApiResponse("sessions.command", result);
      } catch (error) {
        if (error instanceof SdkError) {
          throw error;
        }

        const aborted = options?.signal?.aborted === true;
        if (aborted || timedOut) {
          throw new SdkError({
            code: timedOut ? "timeout" : "aborted",
            message: timedOut
              ? `Request timed out for sessions.command`
              : `Request aborted for sessions.command`,
            source: "sessions.command",
            details: error,
          });
        }

        throw new SdkError({
          code: "api_error",
          message: normalizeUnknownError(error),
          source: "sessions.command",
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
    },

    async history() {
      const result = await sessionCommandHistory({
        client,
      });
      return unwrapApiResponse("sessions.history", result);
    },

    async context(query) {
      const result = await sessionContext({
        client,
        query,
      });
      return unwrapApiResponse("sessions.context", result);
    },
  };
}
