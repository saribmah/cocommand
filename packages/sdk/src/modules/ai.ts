import type { Client } from "../client";

export interface GenerateOptions {
  prompt: string;
  system?: string;
  temperature?: number;
  maxTokens?: number;
}

export interface GenerateResult {
  text: string;
}

export interface AIApi {
  generate(options: GenerateOptions): Promise<GenerateResult>;
}

export function createAI(_client: Client): AIApi {
  return {
    async generate(_options) {
      throw new Error(
        "@cocommand/sdk: AI.generate() is not yet implemented. " +
        "Requires POST /ai/generate Rust endpoint.",
      );
    },
  };
}
