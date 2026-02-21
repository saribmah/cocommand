import type { Client, CoreEvent, Message, MessagePart, SessionContext } from "@cocommand/api";
import { fetchSseGet } from "./request";
import { readSse } from "./sse";

export type RuntimeEvent =
  | {
      type: "message.started";
      sessionId: string;
      runId: string;
      userMessage?: Message;
      assistantMessage: Message;
    }
  | {
      type: "part.updated";
      sessionId: string;
      runId: string;
      messageId: string;
      partId: string;
      part: MessagePart;
    }
  | {
      type: "run.completed";
      sessionId: string;
      runId: string;
    }
  | {
      type: "run.cancelled";
      sessionId: string;
      runId: string;
      reason: string;
    }
  | {
      type: "background-job.started";
      sessionId: string;
      runId: string;
      toolCallId: string;
      toolName: string;
      jobId: string;
    }
  | {
      type: "background-job.completed";
      sessionId: string;
      runId: string;
      toolCallId: string;
      toolName: string;
      jobId: string;
    }
  | {
      type: "background-job.failed";
      sessionId: string;
      runId: string;
      toolCallId: string;
      toolName: string;
      jobId: string;
      error: string;
    }
  | {
      type: "context";
      sessionId: string;
      runId?: string;
      context: SessionContext;
    };

export interface EventsApi {
  stream(options?: {
    signal?: AbortSignal;
    timeoutMs?: number;
    sessionId?: string;
  }): AsyncGenerator<RuntimeEvent>;
}

export function createEventsApi(client: Client): EventsApi {
  return {
    async *stream(options) {
      const query = options?.sessionId
        ? `?session_id=${encodeURIComponent(options.sessionId)}`
        : "";
      const response = await fetchSseGet(client, `/events${query}`, options);

      for await (const event of readSse(response)) {
        const payload = event.data as CoreEvent;
        const mapped = mapCoreEvent(payload);
        if (mapped) {
          yield mapped;
        }
      }
    },
  };
}

function mapCoreEvent(event: CoreEvent): RuntimeEvent | null {
  switch (event.type) {
    case "SessionMessageStarted":
      return {
        type: "message.started",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
        userMessage: event.payload.user_message ?? undefined,
        assistantMessage: event.payload.assistant_message,
      };
    case "SessionPartUpdated":
      return {
        type: "part.updated",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
        messageId: event.payload.message_id,
        partId: event.payload.part_id,
        part: event.payload.part,
      };
    case "SessionRunCompleted":
      return {
        type: "run.completed",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
      };
    case "SessionRunCancelled":
      return {
        type: "run.cancelled",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
        reason: event.payload.reason,
      };
    case "BackgroundJobStarted":
      return {
        type: "background-job.started",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
        toolCallId: event.payload.tool_call_id,
        toolName: event.payload.tool_name,
        jobId: event.payload.job_id,
      };
    case "BackgroundJobCompleted":
      return {
        type: "background-job.completed",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
        toolCallId: event.payload.tool_call_id,
        toolName: event.payload.tool_name,
        jobId: event.payload.job_id,
      };
    case "BackgroundJobFailed":
      return {
        type: "background-job.failed",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id,
        toolCallId: event.payload.tool_call_id,
        toolName: event.payload.tool_name,
        jobId: event.payload.job_id,
        error: event.payload.error,
      };
    case "SessionContextUpdated":
      return {
        type: "context",
        sessionId: event.payload.session_id,
        runId: event.payload.run_id ?? undefined,
        context: event.payload.context,
      };
    default:
      return null;
  }
}
