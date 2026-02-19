import { createApiClient } from "@cocommand/api";
import { invokeTool } from "@cocommand/api";

export interface InvokeResponse<T = unknown> {
  ok: boolean;
  data?: T;
  error?: { code: string; message: string };
}

export async function invokeExtensionTool<T = unknown>(
  addr: string,
  extensionId: string,
  toolId: string,
  input: Record<string, unknown> = {},
  options?: { signal?: AbortSignal },
): Promise<T> {
  const client = createApiClient(addr);
  const { data, error: fetchError, response } = await invokeTool({
    client,
    path: { extension_id: extensionId, tool_id: toolId },
    body: input as Record<string, never>,
    signal: options?.signal,
  });
  if (fetchError || !response.ok) {
    const errBody = fetchError as { error?: { message?: string } } | undefined;
    throw new Error(errBody?.error?.message ?? `Server error (${response.status})`);
  }
  const result = data as unknown as InvokeResponse<T>;
  if (!result.ok) {
    throw new Error(result.error?.message ?? "Unknown error");
  }
  return result.data as T;
}
