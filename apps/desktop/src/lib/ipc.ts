// Re-export the canonical bridge contract from types/core.
// All backend integration goes through these invoke wrappers.
export {
  submitCommand,
  confirmAction,
  getRecentActions,
  getWorkspaceSnapshot,
  hideWindow,
  normalizeResponse,
  getServerInfo,
} from "../types/core";

export type {
  CoreResponse,
  CoreResult,
  ArtifactAction,
  ActionSummary,
  Workspace,
  ArtifactResult,
  PreviewResult,
  ConfirmationResult,
  ErrorResult,
  ServerInfo,
} from "../types/core";
