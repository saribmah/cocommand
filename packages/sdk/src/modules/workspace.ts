import type { Transport } from "../transport";
import type { WorkspaceConfig } from "../types";

export interface WorkspaceApi {
  getConfig(): Promise<WorkspaceConfig>;
  updateConfig(updates: Partial<WorkspaceConfig>): Promise<WorkspaceConfig>;
}

export function createWorkspace(t: Transport): WorkspaceApi {
  return {
    async getConfig() {
      return t.apiGet<WorkspaceConfig>("/workspace/config");
    },
    async updateConfig(updates) {
      return t.apiPost<WorkspaceConfig>("/workspace/config", updates);
    },
  };
}
