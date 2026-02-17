export type { ComposerActionsBridge } from "./configure";

export { createTransport } from "./transport";
export type { Transport } from "./transport";

export type {
  TextSource,
  FilePartInput,
  ExtensionPartInput,
  Application,
  WorkspaceConfig,
  ClipboardSnapshot,
  ClipboardEntry,
  InvokeResponse,
} from "./types";

export { createClipboard } from "./modules/clipboard";
export type { ClipboardApi } from "./modules/clipboard";

export { createApplications } from "./modules/applications";
export type { ApplicationsApi } from "./modules/applications";

export { createWorkspace } from "./modules/workspace";
export type { WorkspaceApi } from "./modules/workspace";

export { Cache } from "./modules/cache";

export { createComposer } from "./modules/composer";
export type { ComposerApi } from "./modules/composer";

export { createAI } from "./modules/ai";
export type { GenerateOptions, GenerateResult, AIApi } from "./modules/ai";

export { createLocalStorage } from "./modules/local-storage";
export type { LocalStorageApi } from "./modules/local-storage";

export { createWindowManagement } from "./modules/window";
export type { ToastOptions, WindowManagementApi } from "./modules/window";

export { createApi } from "./create-api";
export type { CocommandApi, CreateApiOptions } from "./create-api";

export { ApiProvider, useApi } from "./react";
export type { ApiProviderProps } from "./react";
