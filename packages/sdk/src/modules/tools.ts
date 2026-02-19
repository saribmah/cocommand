import type { Client } from "../client";
import { invokeToolUnwrap } from "../client";

export interface ToolsApi {
  invoke<T>(toolId: string, input?: Record<string, unknown>): Promise<T>;
}

export function createTools(client: Client, extensionId: string): ToolsApi {
  return {
    invoke<T>(toolId: string, input?: Record<string, unknown>) {
      return invokeToolUnwrap<T>(client, extensionId, toolId, input);
    },
  };
}
