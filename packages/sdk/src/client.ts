export { createApiClient, type Client } from "@cocommand/api";
import { invokeTool } from "@cocommand/api";
import type { Client } from "@cocommand/api";
import type { InvokeResponse } from "./types";

/**
 * Call `invokeTool` via the API client and unwrap the `{ ok, data, error }` envelope.
 * Throws on HTTP errors and on `ok: false` responses.
 */
export async function invokeToolUnwrap<T>(
  client: Client,
  extensionId: string,
  toolId: string,
  input: Record<string, unknown> = {},
): Promise<T> {
  const { data, error } = await invokeTool({
    client,
    path: { extension_id: extensionId, tool_id: toolId },
    body: input,
  });

  if (error) {
    throw new Error(
      (error as { error?: { message?: string } }).error?.message ??
        "Tool invocation failed",
    );
  }

  const envelope = data as InvokeResponse<T> | undefined;
  if (!envelope || !envelope.ok) {
    throw new Error(
      envelope?.error?.message ?? "Tool invocation returned ok: false",
    );
  }
  return envelope.data as T;
}
