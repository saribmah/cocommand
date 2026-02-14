import { create } from "zustand";
import type { WorkspaceLlmSettings } from "../workspace/workspace.types";

export type SettingsTab = "overview" | "workspace" | "llm";
export type SettingsToast = null | "success" | "error";

export interface SettingsLlmForm {
  provider: string;
  base_url: string;
  model: string;
  system_prompt: string;
  temperature: string;
  max_output_tokens: string;
  max_steps: string;
  api_key: string;
}

const DEFAULT_LLM_FORM: SettingsLlmForm = {
  provider: "openai-compatible",
  base_url: "",
  model: "",
  system_prompt: "",
  temperature: "0.7",
  max_output_tokens: "80000",
  max_steps: "8",
  api_key: "",
};

export interface SettingsState {
  tab: SettingsTab;
  llmForm: SettingsLlmForm;
  llmSaving: boolean;
  llmToast: SettingsToast;
  setTab: (tab: SettingsTab) => void;
  setLlmField: (field: keyof SettingsLlmForm, value: string) => void;
  syncLlmFormFromWorkspace: (settings: WorkspaceLlmSettings) => void;
  clearLlmApiKeyInput: () => void;
  setLlmSaving: (saving: boolean) => void;
  setLlmToast: (toast: SettingsToast) => void;
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;

export const createSettingsStore = () => {
  return create<SettingsState>()((set) => ({
    tab: "overview",
    llmForm: DEFAULT_LLM_FORM,
    llmSaving: false,
    llmToast: null,

    setTab: (tab) => set({ tab }),

    setLlmField: (field, value) =>
      set((state) => ({
        llmForm: {
          ...state.llmForm,
          [field]: value,
        },
      })),

    syncLlmFormFromWorkspace: (settings) =>
      set({
        llmForm: {
          provider: settings.provider,
          base_url: settings.base_url,
          model: settings.model,
          system_prompt: settings.system_prompt,
          temperature: String(settings.temperature ?? 0.7),
          max_output_tokens: String(settings.max_output_tokens ?? 80000),
          max_steps: String(settings.max_steps ?? 8),
          api_key: "",
        },
      }),

    clearLlmApiKeyInput: () =>
      set((state) => ({
        llmForm: {
          ...state.llmForm,
          api_key: "",
        },
      })),

    setLlmSaving: (llmSaving) => set({ llmSaving }),

    setLlmToast: (llmToast) => set({ llmToast }),
  }));
};
