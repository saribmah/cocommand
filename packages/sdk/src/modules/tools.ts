import type { Transport } from "../transport";

export interface ToolsApi {
  invoke<T>(toolId: string, input?: Record<string, unknown>): Promise<T>;
}

export function createTools(t: Transport, extensionId: string): ToolsApi {
  return {
    invoke<T>(toolId: string, input?: Record<string, unknown>) {
      return t.invokeTool<T>(extensionId, toolId, input);
    },
  };
}
