import type { Client, EnqueueMessageResponse, SessionCommandInputPart } from "@cocommand/api";
import { Cache } from "./cache";
import type { ComposerActionsBridge } from "./configure";
import { createComposerApi, type ComposerApi } from "./composer";
import {
  createDeferredAI,
  createDeferredLocalStorage,
  createDeferredUiApi,
  type AIApi,
  type LocalStorageApi,
  type UiApi,
} from "./deferred";
import { createApplicationsApi, type ApplicationsApi } from "./applications";
import { createExtensionsApi, type ExtensionsApi } from "./extensions";
import { createToolsApi, type ToolsApi } from "./tools";
import { createEventsApi, type EventsApi } from "./events";
import { createSessionsApi, type SessionsApi } from "./sessions";
import {
  createBrowserApi,
  createClipboardApi,
  createFilesystemApi,
  createNotesApi,
  createScreenshotApi,
  createSystemApi,
  createWorkspaceApi,
  type BrowserApi,
  type ClipboardApi,
  type FilesystemApi,
  type NotesApi,
  type ScreenshotApi,
  type SystemApi,
  type WorkspaceApi,
} from "./builtins";
import { createOAuthApi, type OAuthApi } from "./oauth";

export interface ExtensionToolsApi {
  invoke<T>(
    toolId: string,
    input?: Record<string, unknown>,
    options?: { signal?: AbortSignal; timeoutMs?: number },
  ): Promise<T>;
}

export interface ExtensionSdk {
  extensionId: string;
  tools: ExtensionToolsApi;
  oauth: OAuthApi;
  composer: ComposerApi;
  ai: AIApi;
  localStorage: LocalStorageApi;
  ui: UiApi;
  cache: Cache;
}

export interface Sdk {
  client: Client;
  tools: ToolsApi;
  extensions: ExtensionsApi;
  applications: ApplicationsApi;
  sessions: SessionsApi;
  events: EventsApi;

  clipboard: ClipboardApi;
  workspace: WorkspaceApi;
  browser: BrowserApi;
  system: SystemApi;
  screenshots: ScreenshotApi;
  filesystem: FilesystemApi;
  notes: NotesApi;

  ui: UiApi;

  command(
    parts: SessionCommandInputPart[],
    options?: { signal?: AbortSignal; timeoutMs?: number },
  ): Promise<EnqueueMessageResponse>;

  extension(extensionId: string, options?: { composer?: ComposerActionsBridge }): ExtensionSdk;
}

export interface CreateSdkOptions {
  client: Client;
}

export function createSdk({ client }: CreateSdkOptions): Sdk {
  const tools = createToolsApi(client);
  const sessions = createSessionsApi(client);

  const sdk: Sdk = {
    client,
    tools,
    extensions: createExtensionsApi(client),
    applications: createApplicationsApi(client),
    sessions,
    events: createEventsApi(client),

    clipboard: createClipboardApi(client),
    workspace: createWorkspaceApi(client),
    browser: createBrowserApi(client),
    system: createSystemApi(client),
    screenshots: createScreenshotApi(client),
    filesystem: createFilesystemApi(client),
    notes: createNotesApi(client),

    ui: createDeferredUiApi(),

    command(parts, options) {
      return sessions.command(parts, options);
    },

    extension(extensionId, options) {
      return createExtensionSdk({
        sdk,
        extensionId,
        composer: options?.composer,
      });
    },
  };

  return sdk;
}

interface CreateExtensionSdkOptions {
  sdk: Sdk;
  extensionId: string;
  composer?: ComposerActionsBridge;
}

export function createExtensionSdk(opts: CreateExtensionSdkOptions): ExtensionSdk {
  const { sdk, extensionId, composer } = opts;

  return {
    extensionId,
    tools: {
      invoke<T>(toolId: string, input: Record<string, unknown> = {}, options?: { signal?: AbortSignal; timeoutMs?: number }) {
        return sdk.tools.invoke<T>(extensionId, toolId, input, options);
      },
    },
    oauth: createOAuthApi(sdk.client, extensionId),
    composer: createComposerApi(composer),
    ai: createDeferredAI(),
    localStorage: createDeferredLocalStorage(extensionId),
    ui: createDeferredUiApi(),
    cache: new Cache(extensionId),
  };
}
