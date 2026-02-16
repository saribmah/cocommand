export interface ExtensionToolInfo {
  id: string;
  name: string;
  description?: string | null;
  input_schema: unknown;
}

export interface ExtensionViewInfo {
  entry: string;
  label: string;
  popout?: { width: number; height: number; title: string };
}

export interface ExtensionInfo {
  id: string;
  name: string;
  kind: string;
  tags: string[];
  tools: ExtensionToolInfo[];
  view?: ExtensionViewInfo;
}

export type ExtensionInvokeFn = <T = unknown>(
  extensionId: string,
  toolId: string,
  input?: Record<string, unknown>,
  options?: { signal?: AbortSignal },
) => Promise<T>;
