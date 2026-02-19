import type { MessagePart, SessionCommandInputPart } from "@cocommand/api";

export type * from "@cocommand/api";

export type TextPartInput = Extract<SessionCommandInputPart, { type: "text" }>;
export type ExtensionPartInput = Extract<SessionCommandInputPart, { type: "extension" }>;
export type FilePartInput = Extract<SessionCommandInputPart, { type: "file" }>;
export type MessagePartInput = SessionCommandInputPart;

export type TextPart = Extract<MessagePart, { type: "text" }>;
export type ReasoningPart = Extract<MessagePart, { type: "reasoning" }>;
export type ToolPart = Extract<MessagePart, { type: "tool" }>;
export type ExtensionPart = Extract<MessagePart, { type: "extension" }>;
export type FilePart = Extract<MessagePart, { type: "file" }>;
