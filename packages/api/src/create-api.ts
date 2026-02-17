import type { Transport } from "./transport";
import type { ComposerActionsBridge } from "./configure";
import { createClipboard, type ClipboardApi } from "./modules/clipboard";
import { createApplications, type ApplicationsApi } from "./modules/applications";
import { createWorkspace, type WorkspaceApi } from "./modules/workspace";
import { createComposer, type ComposerApi } from "./modules/composer";
import { createAI, type AIApi } from "./modules/ai";
import { createLocalStorage, type LocalStorageApi } from "./modules/local-storage";
import { createWindowManagement, type WindowManagementApi, type ToastOptions } from "./modules/window";
import { Cache } from "./modules/cache";

export interface CocommandApi {
  clipboard: ClipboardApi;
  applications: ApplicationsApi;
  workspace: WorkspaceApi;
  composer: ComposerApi;
  ai: AIApi;
  localStorage: LocalStorageApi;
  showToast: (options: ToastOptions) => Promise<void>;
  windowManagement: WindowManagementApi;
  cache: Cache;
}

export interface CreateApiOptions {
  transport: Transport;
  extensionId: string;
  composer?: ComposerActionsBridge;
}

export function createApi(opts: CreateApiOptions): CocommandApi {
  const { transport, extensionId, composer } = opts;
  const win = createWindowManagement(transport);

  return {
    clipboard: createClipboard(transport),
    applications: createApplications(transport),
    workspace: createWorkspace(transport),
    composer: createComposer(composer),
    ai: createAI(transport),
    localStorage: createLocalStorage(transport, extensionId),
    showToast: win.showToast,
    windowManagement: win.windowManagement,
    cache: new Cache(extensionId),
  };
}
