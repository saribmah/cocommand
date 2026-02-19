import { status } from "@cocommand/api";
import type {
  BrowserGetActiveTabInput,
  BrowserGetActiveTabOutput,
  BrowserGetContentInput,
  BrowserGetContentOutput,
  BrowserGetTabsInput,
  BrowserGetTabsOutput,
  BrowserStatus,
  ClipboardClearClipboardHistoryInput,
  ClipboardClearClipboardHistoryOutput,
  ClipboardGetClipboardInput,
  ClipboardGetClipboardOutput,
  ClipboardListClipboardHistoryInput,
  ClipboardListClipboardHistoryOutput,
  ClipboardRecordClipboardInput,
  ClipboardRecordClipboardOutput,
  ClipboardSetClipboardInput,
  ClipboardSetClipboardOutput,
  FilesystemGetIconsInput,
  FilesystemGetIconsOutput,
  FilesystemIndexStatusInput,
  FilesystemIndexStatusOutput,
  FilesystemListDirectoryInput,
  FilesystemListDirectoryOutput,
  FilesystemOpenPathInput,
  FilesystemOpenPathOutput,
  FilesystemPathInfoInput,
  FilesystemPathInfoOutput,
  FilesystemReadFileInput,
  FilesystemReadFileOutput,
  FilesystemRescanIndexInput,
  FilesystemRescanIndexOutput,
  FilesystemRevealPathInput,
  FilesystemRevealPathOutput,
  FilesystemSearchInput,
  FilesystemSearchOutput,
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
  ScreenshotCaptureScreenshotInput,
  ScreenshotCaptureScreenshotOutput,
  ScreenshotCopyScreenshotToClipboardInput,
  ScreenshotCopyScreenshotToClipboardOutput,
  ScreenshotDeleteScreenshotInput,
  ScreenshotDeleteScreenshotOutput,
  ScreenshotGetScreenshotInput,
  ScreenshotGetScreenshotOutput,
  ScreenshotListScreenshotsInput,
  ScreenshotListScreenshotsOutput,
  SystemAppActionInput,
  SystemAppActionOutput,
  SystemListInstalledAppsInput,
  SystemListInstalledAppsOutput,
  SystemListOpenAppsInput,
  SystemListOpenAppsOutput,
  SystemListWindowsInput,
  SystemListWindowsOutput,
  SystemRunApplescriptInput,
  SystemRunApplescriptOutput,
  SystemWindowActionInput,
  SystemWindowActionOutput,
  WorkspaceGetConfigInput,
  WorkspaceGetConfigOutput,
  WorkspaceGetPermissionsInput,
  WorkspaceGetPermissionsOutput,
  WorkspaceOpenPermissionInput,
  WorkspaceOpenPermissionOutput,
  WorkspaceUpdateConfigInput,
  WorkspaceUpdateConfigOutput,
  Client,
} from "@cocommand/api";
import type { RequestOptions } from "./client";
import { invokeExtensionTool } from "./client";
import { unwrapApiResponse } from "./request";

export interface ClipboardApi {
  get(input?: ClipboardGetClipboardInput, options?: RequestOptions): Promise<ClipboardGetClipboardOutput>;
  set(input: ClipboardSetClipboardInput, options?: RequestOptions): Promise<ClipboardSetClipboardOutput>;
  setText(text: string, options?: RequestOptions): Promise<ClipboardSetClipboardOutput>;
  setImage(imagePath: string, options?: RequestOptions): Promise<ClipboardSetClipboardOutput>;
  setFiles(files: string[], options?: RequestOptions): Promise<ClipboardSetClipboardOutput>;
  record(input?: ClipboardRecordClipboardInput, options?: RequestOptions): Promise<ClipboardRecordClipboardOutput>;
  listHistory(input?: ClipboardListClipboardHistoryInput, options?: RequestOptions): Promise<ClipboardListClipboardHistoryOutput>;
  clearHistory(input?: ClipboardClearClipboardHistoryInput, options?: RequestOptions): Promise<ClipboardClearClipboardHistoryOutput>;
}

export interface WorkspaceApi {
  getConfig(input?: WorkspaceGetConfigInput, options?: RequestOptions): Promise<WorkspaceGetConfigOutput>;
  updateConfig(input: WorkspaceUpdateConfigInput, options?: RequestOptions): Promise<WorkspaceUpdateConfigOutput>;
  getPermissions(input?: WorkspaceGetPermissionsInput, options?: RequestOptions): Promise<WorkspaceGetPermissionsOutput>;
  openPermission(input: WorkspaceOpenPermissionInput, options?: RequestOptions): Promise<WorkspaceOpenPermissionOutput>;
}

export interface BrowserApi {
  status(options?: RequestOptions): Promise<BrowserStatus>;
  getTabs(input?: BrowserGetTabsInput, options?: RequestOptions): Promise<BrowserGetTabsOutput>;
  getActiveTab(input?: BrowserGetActiveTabInput, options?: RequestOptions): Promise<BrowserGetActiveTabOutput>;
  getContent(input?: BrowserGetContentInput, options?: RequestOptions): Promise<BrowserGetContentOutput>;
}

export interface SystemApi {
  listOpenApps(input?: SystemListOpenAppsInput, options?: RequestOptions): Promise<SystemListOpenAppsOutput>;
  listWindows(input?: SystemListWindowsInput, options?: RequestOptions): Promise<SystemListWindowsOutput>;
  runAppleScript(input: SystemRunApplescriptInput, options?: RequestOptions): Promise<SystemRunApplescriptOutput>;
  listInstalledApps(input?: SystemListInstalledAppsInput, options?: RequestOptions): Promise<SystemListInstalledAppsOutput>;
  appAction(input: SystemAppActionInput, options?: RequestOptions): Promise<SystemAppActionOutput>;
  windowAction(input: SystemWindowActionInput, options?: RequestOptions): Promise<SystemWindowActionOutput>;
}

export interface ScreenshotApi {
  capture(input?: ScreenshotCaptureScreenshotInput, options?: RequestOptions): Promise<ScreenshotCaptureScreenshotOutput>;
  list(input?: ScreenshotListScreenshotsInput, options?: RequestOptions): Promise<ScreenshotListScreenshotsOutput>;
  get(input: ScreenshotGetScreenshotInput, options?: RequestOptions): Promise<ScreenshotGetScreenshotOutput>;
  remove(input: ScreenshotDeleteScreenshotInput, options?: RequestOptions): Promise<ScreenshotDeleteScreenshotOutput>;
  copyToClipboard(input: ScreenshotCopyScreenshotToClipboardInput, options?: RequestOptions): Promise<ScreenshotCopyScreenshotToClipboardOutput>;
}

export interface FilesystemApi {
  search(input: FilesystemSearchInput, options?: RequestOptions): Promise<FilesystemSearchOutput>;
  listDirectory(input?: FilesystemListDirectoryInput, options?: RequestOptions): Promise<FilesystemListDirectoryOutput>;
  indexStatus(input?: FilesystemIndexStatusInput, options?: RequestOptions): Promise<FilesystemIndexStatusOutput>;
  rescanIndex(input?: FilesystemRescanIndexInput, options?: RequestOptions): Promise<FilesystemRescanIndexOutput>;
  readFile(input: FilesystemReadFileInput, options?: RequestOptions): Promise<FilesystemReadFileOutput>;
  pathInfo(input: FilesystemPathInfoInput, options?: RequestOptions): Promise<FilesystemPathInfoOutput>;
  openPath(input: FilesystemOpenPathInput, options?: RequestOptions): Promise<FilesystemOpenPathOutput>;
  revealPath(input: FilesystemRevealPathInput, options?: RequestOptions): Promise<FilesystemRevealPathOutput>;
  getIcons(input: FilesystemGetIconsInput, options?: RequestOptions): Promise<FilesystemGetIconsOutput>;
}

export interface NotesApi {
  create(input?: NotesCreateNoteInput, options?: RequestOptions): Promise<NotesCreateNoteOutput>;
  list(input?: NotesListNotesInput, options?: RequestOptions): Promise<NotesListNotesOutput>;
  read(input: NotesReadNoteInput, options?: RequestOptions): Promise<NotesReadNoteOutput>;
  update(input: NotesUpdateNoteInput, options?: RequestOptions): Promise<NotesUpdateNoteOutput>;
  remove(input: NotesDeleteNoteInput, options?: RequestOptions): Promise<NotesDeleteNoteOutput>;
  search(input: NotesSearchNotesInput, options?: RequestOptions): Promise<NotesSearchNotesOutput>;
  indexStatus(input?: NotesIndexStatusInput, options?: RequestOptions): Promise<NotesIndexStatusOutput>;
  rescanIndex(input?: NotesRescanIndexInput, options?: RequestOptions): Promise<NotesRescanIndexOutput>;
}

function invoke<T>(
  client: Client,
  extensionId: string,
  toolId: string,
  input: Record<string, unknown>,
  options?: RequestOptions,
): Promise<T> {
  return invokeExtensionTool<T>(client, extensionId, toolId, input, options);
}

export function createClipboardApi(client: Client): ClipboardApi {
  const ext = "clipboard";
  return {
    get(input = {}, options) {
      return invoke<ClipboardGetClipboardOutput>(client, ext, "get_clipboard", input, options);
    },
    set(input, options) {
      return invoke<ClipboardSetClipboardOutput>(client, ext, "set_clipboard", input, options);
    },
    setText(text, options) {
      return invoke<ClipboardSetClipboardOutput>(client, ext, "set_clipboard", { kind: "text", text }, options);
    },
    setImage(imagePath, options) {
      return invoke<ClipboardSetClipboardOutput>(client, ext, "set_clipboard", { kind: "image", imagePath }, options);
    },
    setFiles(files, options) {
      return invoke<ClipboardSetClipboardOutput>(client, ext, "set_clipboard", { kind: "files", files }, options);
    },
    record(input = {}, options) {
      return invoke<ClipboardRecordClipboardOutput>(client, ext, "record_clipboard", input, options);
    },
    listHistory(input = {}, options) {
      return invoke<ClipboardListClipboardHistoryOutput>(client, ext, "list_clipboard_history", input, options);
    },
    clearHistory(input = {}, options) {
      return invoke<ClipboardClearClipboardHistoryOutput>(client, ext, "clear_clipboard_history", input, options);
    },
  };
}

export function createWorkspaceApi(client: Client): WorkspaceApi {
  const ext = "workspace";
  return {
    getConfig(input = {}, options) {
      return invoke<WorkspaceGetConfigOutput>(client, ext, "get_config", input, options);
    },
    updateConfig(input, options) {
      return invoke<WorkspaceUpdateConfigOutput>(client, ext, "update_config", input, options);
    },
    getPermissions(input = {}, options) {
      return invoke<WorkspaceGetPermissionsOutput>(client, ext, "get_permissions", input, options);
    },
    openPermission(input, options) {
      return invoke<WorkspaceOpenPermissionOutput>(client, ext, "open_permission", input, options);
    },
  };
}

export function createBrowserApi(client: Client): BrowserApi {
  const ext = "browser";
  return {
    async status(options) {
      const result = await status({ client, signal: options?.signal });
      return unwrapApiResponse("browser.status", result);
    },
    getTabs(input = {}, options) {
      return invoke<BrowserGetTabsOutput>(client, ext, "get_tabs", input, options);
    },
    getActiveTab(input = {}, options) {
      return invoke<BrowserGetActiveTabOutput>(client, ext, "get_active_tab", input, options);
    },
    getContent(input = {}, options) {
      return invoke<BrowserGetContentOutput>(client, ext, "get_content", input, options);
    },
  };
}

export function createSystemApi(client: Client): SystemApi {
  const ext = "system";
  return {
    listOpenApps(input = {}, options) {
      return invoke<SystemListOpenAppsOutput>(client, ext, "list_open_apps", input, options);
    },
    listWindows(input = {}, options) {
      return invoke<SystemListWindowsOutput>(client, ext, "list_windows", input, options);
    },
    runAppleScript(input, options) {
      return invoke<SystemRunApplescriptOutput>(client, ext, "run_applescript", input, options);
    },
    listInstalledApps(input = {}, options) {
      return invoke<SystemListInstalledAppsOutput>(client, ext, "list_installed_apps", input, options);
    },
    appAction(input, options) {
      return invoke<SystemAppActionOutput>(client, ext, "app_action", input, options);
    },
    windowAction(input, options) {
      return invoke<SystemWindowActionOutput>(client, ext, "window_action", input, options);
    },
  };
}

export function createScreenshotApi(client: Client): ScreenshotApi {
  const ext = "screenshot";
  return {
    capture(input = {}, options) {
      return invoke<ScreenshotCaptureScreenshotOutput>(client, ext, "capture_screenshot", input, options);
    },
    list(input = {}, options) {
      return invoke<ScreenshotListScreenshotsOutput>(client, ext, "list_screenshots", input, options);
    },
    get(input, options) {
      return invoke<ScreenshotGetScreenshotOutput>(client, ext, "get_screenshot", input, options);
    },
    remove(input, options) {
      return invoke<ScreenshotDeleteScreenshotOutput>(client, ext, "delete_screenshot", input, options);
    },
    copyToClipboard(input, options) {
      return invoke<ScreenshotCopyScreenshotToClipboardOutput>(client, ext, "copy_screenshot_to_clipboard", input, options);
    },
  };
}

export function createFilesystemApi(client: Client): FilesystemApi {
  const ext = "filesystem";
  return {
    search(input, options) {
      return invoke<FilesystemSearchOutput>(client, ext, "search", input, options);
    },
    listDirectory(input = {}, options) {
      return invoke<FilesystemListDirectoryOutput>(client, ext, "list_directory", input, options);
    },
    indexStatus(input = {}, options) {
      return invoke<FilesystemIndexStatusOutput>(client, ext, "index_status", input, options);
    },
    rescanIndex(input = {}, options) {
      return invoke<FilesystemRescanIndexOutput>(client, ext, "rescan_index", input, options);
    },
    readFile(input, options) {
      return invoke<FilesystemReadFileOutput>(client, ext, "read_file", input, options);
    },
    pathInfo(input, options) {
      return invoke<FilesystemPathInfoOutput>(client, ext, "path_info", input, options);
    },
    openPath(input, options) {
      return invoke<FilesystemOpenPathOutput>(client, ext, "open_path", input, options);
    },
    revealPath(input, options) {
      return invoke<FilesystemRevealPathOutput>(client, ext, "reveal_path", input, options);
    },
    getIcons(input, options) {
      return invoke<FilesystemGetIconsOutput>(client, ext, "get_icons", input, options);
    },
  };
}

export function createNotesApi(client: Client): NotesApi {
  const ext = "notes";
  return {
    create(input = {}, options) {
      return invoke<NotesCreateNoteOutput>(client, ext, "create-note", input, options);
    },
    list(input = {}, options) {
      return invoke<NotesListNotesOutput>(client, ext, "list-notes", input, options);
    },
    read(input, options) {
      return invoke<NotesReadNoteOutput>(client, ext, "read-note", input, options);
    },
    update(input, options) {
      return invoke<NotesUpdateNoteOutput>(client, ext, "update-note", input, options);
    },
    remove(input, options) {
      return invoke<NotesDeleteNoteOutput>(client, ext, "delete-note", input, options);
    },
    search(input, options) {
      return invoke<NotesSearchNotesOutput>(client, ext, "search-notes", input, options);
    },
    indexStatus(input = {}, options) {
      return invoke<NotesIndexStatusOutput>(client, ext, "index-status", input, options);
    },
    rescanIndex(input = {}, options) {
      return invoke<NotesRescanIndexOutput>(client, ext, "rescan-index", input, options);
    },
  };
}
