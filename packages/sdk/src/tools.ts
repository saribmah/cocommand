import type { Client } from "@cocommand/api";
import type { RequestOptions } from "./client";
import { invokeExtensionTool } from "./client";

export interface ToolsApi {
  invoke<T>(
    extensionId: string,
    toolId: string,
    input?: Record<string, unknown>,
    options?: RequestOptions,
  ): Promise<T>;
}

export function createToolsApi(client: Client): ToolsApi {
  return {
    invoke<T>(extensionId: string, toolId: string, input: Record<string, unknown> = {}, options?: RequestOptions) {
      return invokeExtensionTool<T>(client, extensionId, toolId, input, options);
    },
  };
}
