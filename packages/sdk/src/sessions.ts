import type {
  ApiSessionContext,
  Message,
  MessagePart,
  RecordMessageResponse,
  SessionCommandInputPart,
  SessionContextData,
  Client,
} from "@cocommand/api";
import { sessionContext } from "@cocommand/api";
import { SdkError } from "./errors";
import { fetchSse } from "./request";
import { readSse } from "./sse";
import { unwrapApiResponse } from "./request";

export interface SessionCommandOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
}

export type SessionCommandEvent =
  | {
      type: "message.started";
      userMessage: Message;
      assistantMessage: Message;
    }
  | {
      type: "part.updated";
      messageId: string;
      partId: string;
      part: MessagePart;
    }
  | {
      type: "context";
      context: ApiSessionContext;
    }
  | {
      type: "done";
      context: ApiSessionContext;
      messages: Message[];
    };

interface MessageStartedPayload {
  user_message?: Message;
  assistant_message?: Message;
}

interface PartUpdatedPayload {
  message_id?: string;
  part_id?: string;
  part?: MessagePart;
}

interface ContextPayload {
  context?: ApiSessionContext;
}

interface DonePayload {
  context?: ApiSessionContext;
  messages?: Message[];
}

interface ErrorPayload {
  error?: { code?: string; message?: string };
  code?: string;
  message?: string;
}

export interface SessionsApi {
  commandStream(
    parts: SessionCommandInputPart[],
    options?: SessionCommandOptions,
  ): AsyncGenerator<SessionCommandEvent>;
  command(
    parts: SessionCommandInputPart[],
    options?: SessionCommandOptions,
  ): Promise<RecordMessageResponse>;
  context(query?: SessionContextData["query"]): Promise<ApiSessionContext>;
}

export function createSessionsApi(client: Client): SessionsApi {
  return {
    async *commandStream(parts, options) {
      const response = await fetchSse(client, "/sessions/command", { parts }, options);

      for await (const event of readSse(response)) {
        if (event.event === "message.started") {
          const payload = event.data as MessageStartedPayload;
          if (!payload.user_message || !payload.assistant_message) {
            throw new SdkError({
              code: "sse_parse_error",
              message: "Invalid message.started payload",
              source: "sessions.commandStream",
              details: event.data,
            });
          }
          yield {
            type: "message.started",
            userMessage: payload.user_message,
            assistantMessage: payload.assistant_message,
          };
          continue;
        }

        if (event.event === "part.updated") {
          const payload = event.data as PartUpdatedPayload;
          if (!payload.message_id || !payload.part_id || !payload.part) {
            throw new SdkError({
              code: "sse_parse_error",
              message: "Invalid part.updated payload",
              source: "sessions.commandStream",
              details: event.data,
            });
          }
          yield {
            type: "part.updated",
            messageId: payload.message_id,
            partId: payload.part_id,
            part: payload.part,
          };
          continue;
        }

        if (event.event === "context") {
          const payload = event.data as ContextPayload;
          if (!payload.context) {
            throw new SdkError({
              code: "sse_parse_error",
              message: "Invalid context payload",
              source: "sessions.commandStream",
              details: event.data,
            });
          }
          yield {
            type: "context",
            context: payload.context,
          };
          continue;
        }

        if (event.event === "done") {
          const payload = event.data as DonePayload;
          if (!payload.context || !payload.messages) {
            throw new SdkError({
              code: "sse_parse_error",
              message: "Invalid done payload",
              source: "sessions.commandStream",
              details: event.data,
            });
          }
          yield {
            type: "done",
            context: payload.context,
            messages: payload.messages,
          };
          continue;
        }

        if (event.event === "error") {
          const payload = event.data as ErrorPayload;
          const message =
            payload.error?.message ??
            payload.message ??
            (typeof event.data === "string"
              ? event.data
              : "Session command stream returned an error event");
          throw new SdkError({
            code: "sse_error",
            message,
            source: "sessions.commandStream",
            details: event.data,
          });
        }
      }
    },

    async command(parts, options) {
      let final: RecordMessageResponse | null = null;

      for await (const event of this.commandStream(parts, options)) {
        if (event.type === "done") {
          final = {
            context: event.context,
            messages: event.messages,
          };
        }
      }

      if (!final) {
        throw new SdkError({
          code: "sse_error",
          message: "Session command stream ended without a done event",
          source: "sessions.command",
        });
      }

      return final;
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
