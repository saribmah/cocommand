import type { Client } from "./client";
import type { ComposerActionsBridge } from "./configure";
import { createClipboard, type ClipboardApi } from "./modules/clipboard";
import { createApplications, type ApplicationsApi } from "./modules/applications";
import { createWorkspace, type WorkspaceApi } from "./modules/workspace";
import { createComposer, type ComposerApi } from "./modules/composer";
import { createAI, type AIApi } from "./modules/ai";
import { createLocalStorage, type LocalStorageApi } from "./modules/local-storage";
import { createWindowManagement, type WindowManagementApi, type ToastOptions } from "./modules/window";
import { createTools, type ToolsApi } from "./modules/tools";
import { createOAuth, type OAuthApi } from "./modules/oauth";
import { Cache } from "./modules/cache";

export interface CocommandApi {
  tools: ToolsApi;
  clipboard: ClipboardApi;
  applications: ApplicationsApi;
  workspace: WorkspaceApi;
  composer: ComposerApi;
  ai: AIApi;
  localStorage: LocalStorageApi;
  oauth: OAuthApi;
  showToast: (options: ToastOptions) => Promise<void>;
  windowManagement: WindowManagementApi;
  cache: Cache;
}

export interface CreateApiOptions {
  client: Client;
  extensionId: string;
  composer?: ComposerActionsBridge;
}

export function createApi(opts: CreateApiOptions): CocommandApi {
  const { client, extensionId, composer } = opts;
  const win = createWindowManagement(client);

  return {
    tools: createTools(client, extensionId),
    clipboard: createClipboard(client),
    applications: createApplications(client),
    workspace: createWorkspace(client),
    composer: createComposer(composer),
    ai: createAI(client),
    localStorage: createLocalStorage(client, extensionId),
    oauth: createOAuth(client, extensionId),
    showToast: win.showToast,
    windowManagement: win.windowManagement,
    cache: new Cache(extensionId),
  };
}
