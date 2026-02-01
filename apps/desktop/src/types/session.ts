export interface SessionContext {
  workspace_id: string;
  session_id: string;
  started_at: number;
  ended_at: number | null;
}

export interface RecordMessageResponse {
  context: SessionContext;
  reply_parts: MessagePart[];
}

export interface StreamEvent<T = unknown> {
  event: string;
  data: T;
}

export type StreamPart = {
  type: string;
  text?: string;
  toolName?: string;
  toolCallId?: string;
  tool_name?: string;
  tool_call_id?: string;
  input?: unknown;
  output?: unknown;
  error?: unknown;
  reason?: string;
  mediaType?: string;
  media_type?: string;
  name?: string | null;
  url?: string | null;
  title?: string | null;
  sourceType?: string;
  source_type?: string;
};

export type MessagePart =
  | TextPart
  | ReasoningPart
  | ToolCallPart
  | ToolResultPart
  | SourcePart
  | FilePart;

export interface TextPart {
  type: "text";
  text: string;
}

export interface ReasoningPart {
  type: "reasoning";
  text: string;
}

export interface ToolCallPart {
  type: "tool-call";
  call_id: string;
  tool_name: string;
  input: unknown;
}

export interface ToolResultPart {
  type: "tool-result";
  call_id: string;
  tool_name: string;
  output: unknown;
  is_error: boolean;
}

export interface SourcePart {
  type: "source";
  id: string;
  source_type: string;
  url?: string | null;
  title?: string | null;
  media_type?: string | null;
  filename?: string | null;
}

export interface FilePart {
  type: "file";
  base64: string;
  media_type: string;
  name?: string | null;
}
