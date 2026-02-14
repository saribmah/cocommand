export interface WorkspaceTheme {
  mode: string;
  accent: string;
}

export interface WorkspaceExtension {
  extension_id: string;
  version: string;
  enabled: boolean;
}

export interface WorkspaceExtensions {
  installed: WorkspaceExtension[];
}

export interface SessionPreferences {
  rollover_mode: string;
  duration_seconds: number;
}

export interface ExtensionCachePreferences {
  max_extensions: number;
}

export interface FileSystemPreferences {
  watch_root: string;
  ignore_paths: string[];
}

export interface WorkspacePreferences {
  language: string;
  session: SessionPreferences;
  extension_cache: ExtensionCachePreferences;
  filesystem: FileSystemPreferences;
}

export interface WorkspaceLlmSettings {
  provider: string;
  base_url: string;
  api_key: string | null;
  model: string;
  system_prompt: string;
  temperature: number;
  max_output_tokens: number;
  max_steps: number;
}

export interface WorkspaceOnboarding {
  completed: boolean;
  completed_at: number | null;
  version: string;
}

export interface WorkspacePermissionStatus {
  id: string;
  label: string;
  granted: boolean;
  required: boolean;
}

export interface WorkspacePermissionsResponse {
  platform: string;
  permissions: WorkspacePermissionStatus[];
}

export interface WorkspaceConfig {
  version: string;
  workspace_id: string;
  name: string;
  extensions: WorkspaceExtensions;
  preferences: WorkspacePreferences;
  llm: WorkspaceLlmSettings;
  theme: WorkspaceTheme;
  onboarding: WorkspaceOnboarding;
  created_at: number;
  last_modified: number;
}
