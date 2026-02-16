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
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  const url = `${prefix}/extension/${extensionId}/invoke/${toolId}`;
  const response = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
    signal: options?.signal,
  });
  const result = (await response.json()) as InvokeResponse<T>;
  if (!result.ok || !response.ok) {
    throw new Error(result.error?.message ?? `Server error (${response.status})`);
  }
  return result.data as T;
}
