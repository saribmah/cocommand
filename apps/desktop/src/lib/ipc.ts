// Re-export the canonical bridge contract from types/core.
// All backend integration goes through these invoke wrappers.
export {
  hideWindow,
  hideSettingsWindow,
  openSettingsWindow,
  getServerInfo,
} from "../types/core";

export type { CoreResult, ErrorResult, ServerInfo } from "../types/core";
