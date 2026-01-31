export interface AiSettings {
  provider: string;
  base_url: string;
  model: string;
  system_prompt: string;
  temperature: number;
  max_output_tokens: number;
  max_steps: number;
  has_api_key: boolean;
}

export interface UpdateAiSettingsPayload {
  provider?: string;
  base_url?: string;
  api_key?: string;
  model?: string;
  system_prompt?: string;
  temperature?: number;
  max_output_tokens?: number;
  max_steps?: number;
}
