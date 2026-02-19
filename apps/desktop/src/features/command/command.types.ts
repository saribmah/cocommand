import type {
    MessagePart,
    SessionCommandInputPart,
} from "@cocommand/api";

// ---------------------------------------------------------------------------
// Re-exports from @cocommand/api
// ---------------------------------------------------------------------------

export type {
    RecordMessageResponse,
    PartBase as MessagePartBase,
    FilePartSourceText as TextSource,
    ToolState,
    ToolStatePending,
    ToolStateRunning,
    ToolStateCompleted,
    ToolStateError,
    MessagePart
} from "@cocommand/api";

// ---------------------------------------------------------------------------
// Input part types (discriminated variants extracted from the API union)
// ---------------------------------------------------------------------------

export type TextPartInput = Extract<SessionCommandInputPart, { type: "text" }>;
export type ExtensionPartInput = Extract<SessionCommandInputPart, { type: "extension" }>;
export type FilePartInput = Extract<SessionCommandInputPart, { type: "file" }>;
export type MessagePartInput = SessionCommandInputPart;

// ---------------------------------------------------------------------------
// Message part types (discriminated variants extracted from the API union)
// ---------------------------------------------------------------------------

export type TextPart = Extract<MessagePart, { type: "text" }>;
export type ReasoningPart = Extract<MessagePart, { type: "reasoning" }>;
export type ToolPart = Extract<MessagePart, { type: "tool" }>;
export type ExtensionPart = Extract<MessagePart, { type: "extension" }>;
export type FilePart = Extract<MessagePart, { type: "file" }>;
