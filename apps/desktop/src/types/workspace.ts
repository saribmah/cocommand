export interface WorkspaceTheme {
  mode: string;
  accent: string;
}

export interface WorkspaceSettings {
  name: string;
  theme: WorkspaceTheme;
}

export interface UpdateWorkspaceSettingsPayload {
  name?: string;
  theme_mode?: string;
  theme_accent?: string;
}
