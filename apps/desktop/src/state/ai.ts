import { create } from "zustand";
import type { AiSettings, UpdateAiSettingsPayload } from "../types/ai";
import type { WorkspaceConfig } from "../features/workspace/workspace.types";
import { useServerStore } from "./server";

interface AiState {
  settings: AiSettings | null;
  isLoaded: boolean;
  error: string | null;
  fetchSettings: () => Promise<void>;
  updateSettings: (payload: UpdateAiSettingsPayload) => Promise<AiSettings>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

function toAiSettings(config: WorkspaceConfig): AiSettings {
  const llm = config.llm;
  return {
    provider: llm.provider,
    base_url: llm.base_url,
    model: llm.model,
    system_prompt: llm.system_prompt,
    temperature: llm.temperature,
    max_output_tokens: llm.max_output_tokens,
    max_steps: llm.max_steps,
    has_api_key: (llm.api_key ?? "").trim().length > 0,
  };
}

async function fetchWorkspaceConfig(addr: string): Promise<WorkspaceConfig> {
  const url = buildServerUrl(addr, "/workspace/config");
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Server error (${response.status})`);
  }
  return (await response.json()) as WorkspaceConfig;
}

async function saveWorkspaceConfig(
  addr: string,
  config: WorkspaceConfig,
): Promise<WorkspaceConfig> {
  const url = buildServerUrl(addr, "/workspace/config");
  const response = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config),
  });
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `Server error (${response.status})`);
  }
  return (await response.json()) as WorkspaceConfig;
}

export const useAiStore = create<AiState>((set) => ({
  settings: null,
  isLoaded: false,
  error: null,
  fetchSettings: async () => {
    const server = useServerStore.getState().info;
    if (!server) {
      set({ settings: null, isLoaded: false, error: null });
      return;
    }
    try {
      const config = await fetchWorkspaceConfig(server.addr);
      set({ settings: toAiSettings(config), isLoaded: true, error: null });
    } catch (err) {
      set({ settings: null, isLoaded: false, error: String(err) });
    }
  },
  updateSettings: async (payload) => {
    const server = useServerStore.getState().info;
    if (!server) {
      throw new Error("Server unavailable");
    }
    const currentConfig = await fetchWorkspaceConfig(server.addr);
    const apiKey =
      payload.api_key !== undefined
        ? payload.api_key.trim().length > 0
          ? payload.api_key
          : null
        : currentConfig.llm.api_key;
    const nextConfig: WorkspaceConfig = {
      ...currentConfig,
      llm: {
        ...currentConfig.llm,
        provider: payload.provider ?? currentConfig.llm.provider,
        base_url: payload.base_url ?? currentConfig.llm.base_url,
        api_key: apiKey,
        model: payload.model ?? currentConfig.llm.model,
        system_prompt: payload.system_prompt ?? currentConfig.llm.system_prompt,
        temperature: payload.temperature ?? currentConfig.llm.temperature,
        max_output_tokens:
          payload.max_output_tokens ?? currentConfig.llm.max_output_tokens,
        max_steps: payload.max_steps ?? currentConfig.llm.max_steps,
      },
    };
    const savedConfig = await saveWorkspaceConfig(server.addr, nextConfig);
    const settings = toAiSettings(savedConfig);
    set({ settings, isLoaded: true, error: null });
    return settings;
  },
}));
