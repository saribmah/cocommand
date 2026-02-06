export interface ExtensionToolInfo {
  id: string;
  name: string;
  description?: string | null;
  input_schema: unknown;
}

export interface ExtensionInfo {
  id: string;
  name: string;
  kind: string;
  tags: string[];
  tools: ExtensionToolInfo[];
}
