import { invoke } from "@tauri-apps/api/core";

// --- Bridge types (exact match of Rust serde output) ---

export interface ServerInfo {
  addr: string;
  workspace_dir: string;
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
