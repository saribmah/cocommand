import type { Client } from "../client";
import { invokeToolUnwrap } from "../client";
import type { WorkspaceConfig } from "../types";

const EXT = "workspace";

export interface WorkspaceApi {
  getConfig(): Promise<WorkspaceConfig>;
  updateConfig(updates: Partial<WorkspaceConfig>): Promise<WorkspaceConfig>;
}

export function createWorkspace(client: Client): WorkspaceApi {
  return {
    async getConfig() {
      return invokeToolUnwrap<WorkspaceConfig>(client, EXT, "get_config");
    },
    async updateConfig(updates) {
      return invokeToolUnwrap<WorkspaceConfig>(client, EXT, "update_config", {
        config: updates,
      });
    },
  };
}
