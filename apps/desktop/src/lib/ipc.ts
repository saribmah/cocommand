import { invoke } from "@tauri-apps/api/core";

// --- Bridge types (exact match of Rust serde output) ---

export interface ServerInfo {
  status: "starting" | "ready" | "error";
  addr?: string | null;
  workspace_dir: string;
  error?: string | null;
}

// --- Normalized UI types (uniform shape for rendering) ---

export interface ArtifactAction {
  id: string;
  label: string;
}

export interface ArtifactResult {
  type: "artifact";
  title: string;
  body: string;
  actions: ArtifactAction[];
}

export interface PreviewResult {
  type: "preview";
  title: string;
  body: string;
}

export interface ConfirmationResult {
  type: "confirmation";
  title: string;
  body: string;
  confirmation_id: string;
}

export interface ErrorResult {
  type: "error";
  title: string;
  body: string;
}

export type CoreResult = ArtifactResult | PreviewResult | ConfirmationResult | ErrorResult;

// --- Invoke wrappers (only backend integration path) ---

export async function hideWindow(): Promise<void> {
  return invoke("hide_window");
}

export async function getServerInfo(): Promise<ServerInfo> {
  return invoke("get_server_info_cmd");
}

export async function getWorkspaceDir(): Promise<string> {
  return invoke("get_workspace_dir_cmd");
}

export async function setWorkspaceDir(workspaceDir: string): Promise<string> {
  return invoke("set_workspace_dir_cmd", { workspace_dir: workspaceDir });
}

export async function closeExtensionWindow(extensionId: string): Promise<void> {
  return invoke("close_extension_window", { extensionId });
}

export async function openExtensionWindow(params: {
  extensionId: string;
  title: string;
  width: number;
  height: number;
}): Promise<void> {
  return invoke("open_extension_window", {
    extensionId: params.extensionId,
    title: params.title,
    width: params.width,
    height: params.height,
  });
}
