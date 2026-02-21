export { createApiClient as createSdkClient, type Client } from "./client";
export { invokeToolUnwrap, invokeExtensionTool } from "./client";
export type { RequestOptions, InvokeResponse } from "./client";

export { SdkError, SdkNotImplementedError, toSdkError } from "./errors";
export type { SdkErrorCode, SdkErrorOptions } from "./errors";

export { createSdk, createExtensionSdk } from "./sdk";
export type {
  CreateSdkOptions,
  Sdk,
  ExtensionSdk,
  ExtensionToolsApi,
} from "./sdk";

export type {
  ApplicationsApi,
} from "./applications";

export type {
  ExtensionsApi,
  ExtensionViewsApi,
  ResolvedExtensionViewAsset,
} from "./extensions";

export type { ToolsApi } from "./tools";
export type { SessionsApi, SessionCommandOptions } from "./sessions";
export type { EventsApi, RuntimeEvent } from "./events";

export {
  createClipboardApi,
  createWorkspaceApi,
  createBrowserApi,
  createSystemApi,
  createScreenshotApi,
  createFilesystemApi,
  createNotesApi,
} from "./builtins";
export type {
  ClipboardApi,
  WorkspaceApi,
  BrowserApi,
  SystemApi,
  ScreenshotApi,
  FilesystemApi,
  NotesApi,
} from "./builtins";

export { createOAuthApi, PKCEClient, isTokenExpired } from "./oauth";
export type {
  OAuthApi,
  PKCEClientOptions,
  AuthorizationRequest,
  AuthorizationRequestOptions,
  AuthorizationResponse,
  TokenSet,
  TokenSetOptions,
} from "./oauth";

export {
  createDeferredAI,
  createDeferredLocalStorage,
  createDeferredUiApi,
} from "./deferred";
export type {
  AIApi,
  GenerateOptions,
  GenerateResult,
  LocalStorageApi,
  ToastOptions,
  WindowManagementApi,
  UiApi,
} from "./deferred";

export { createComposerApi } from "./composer";
export type { ComposerApi } from "./composer";

export { Cache } from "./cache";

export type { ComposerActionsBridge } from "./configure";

export type {
  ApiErrorBody,
  ApiErrorResponse,
  ApiSessionContext,
  ApplicationInfo,
  OpenApplicationRequest,
  OpenApplicationResponse,
  ApplicationsResponse,
  CoreEvent,
  ExtensionInfo,
  ExtensionToolInfo,
  ExtensionViewInfo,
  ExtensionViewPopoutInfo,
  FilePart,
  FilePartInput,
  FilePartSourceText,
  Message,
  MessageInfo,
  MessagePart,
  MessagePartInput,
  NotesCreateNoteInput,
  NotesCreateNoteOutput,
  NotesDeleteNoteInput,
  NotesDeleteNoteOutput,
  NotesIndexStatusInput,
  NotesIndexStatusOutput,
  NotesListNotesInput,
  NotesListNotesOutput,
  NotesReadNoteInput,
  NotesReadNoteOutput,
  NotesRescanIndexInput,
  NotesRescanIndexOutput,
  NotesSearchNotesInput,
  NotesSearchNotesOutput,
  NotesUpdateNoteInput,
  NotesUpdateNoteOutput,
  PartBase,
  EnqueueMessageResponse,
  SessionCommandInputPart,
  SessionContext,
  ToolState,
  ToolStateCompleted,
  ToolStateError,
  ToolStatePending,
  ToolStateRunning,
  TextPartInput,
} from "./types";

// Backward-compatible alias for older consumers that used createApiClient name.
export { createApiClient } from "./client";
