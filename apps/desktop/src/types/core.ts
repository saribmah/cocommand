import { invoke } from "@tauri-apps/api/core";

// --- Bridge types (exact match of Rust serde output) ---

export interface ArtifactAction {
  id: string;
  label: string;
}

export type CoreResponse =
  | { type: "Artifact"; content: string; actions: ArtifactAction[] }
  | { type: "Preview"; title: string; content: string }
  | { type: "Confirmation"; confirmation_id: string; prompt: string; description: string }
  | { type: "Error"; message: string };

export interface ActionSummary {
  id: string;
  description: string;
}

export type WorkspaceMode = "Idle" | "FollowUpActive" | "AwaitingConfirmation";
export type ApplicationStatus = "Active" | "Inactive";

export interface ApplicationInstance {
  instance_id: string;
  app_id: string;
  status: ApplicationStatus;
  context: Record<string, unknown>;
  mounted_tools: string[];
}

export interface FollowUpContext {
  last_command: string;
  last_result_entity_ids: string[];
  last_app_id: string;
  expires_at: number;
  turn_count: number;
  max_turns: number;
}

export interface ConfirmationPending {
  confirmation_id: string;
  tool_id: string;
  args: unknown;
  requested_at: number;
}

export interface Workspace {
  instances: Record<string, ApplicationInstance>;
  focus: string | null;
  mode: WorkspaceMode;
  follow_up: FollowUpContext | null;
  confirmation_pending: ConfirmationPending | null;
  session_id: string;
  created_at: number;
  last_modified: number;
}

export interface ServerInfo {
  addr: string;
  workspace_dir: string;
}

// --- Normalized UI types (uniform shape for rendering) ---

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

// --- Normalization (CoreResponse → CoreResult for UI rendering) ---

export function normalizeResponse(response: CoreResponse): CoreResult {
  switch (response.type) {
    case "Artifact":
      return {
        type: "artifact",
        title: "Result",
        body: response.content,
        actions: response.actions,
      };
    case "Preview":
      return {
        type: "preview",
        title: response.title,
        body: response.content,
      };
    case "Confirmation":
      return {
        type: "confirmation",
        title: response.prompt,
        body: response.description,
        confirmation_id: response.confirmation_id,
      };
    case "Error":
      return {
        type: "error",
        title: "Error",
        body: response.message,
      };
  }
}

// --- Invoke wrappers (only backend integration path) ---

export async function submitCommand(text: string): Promise<CoreResponse> {
  return invoke("submit_command", { text });
}

export async function confirmAction(
  confirmationId: string,
  decision: boolean
): Promise<CoreResponse> {
  return invoke("confirm_action", {
    confirmation_id: confirmationId,
    decision,
  });
}

export async function getRecentActions(limit: number): Promise<ActionSummary[]> {
  return invoke("get_recent_actions", { limit });
}

export async function getWorkspaceSnapshot(): Promise<Workspace> {
  return invoke("get_workspace_snapshot");
}

export async function hideWindow(): Promise<void> {
  return invoke("hide_window");
}

export async function getServerInfo(): Promise<ServerInfo> {
  return invoke("get_server_info_cmd");
}
