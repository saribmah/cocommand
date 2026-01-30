export interface ApplicationToolInfo {
  id: string;
  name: string;
  description?: string | null;
  input_schema: unknown;
}

export interface ApplicationInfo {
  id: string;
  name: string;
  kind: string;
  tags: string[];
  tools: ApplicationToolInfo[];
}
