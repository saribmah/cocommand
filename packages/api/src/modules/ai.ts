import type { Transport } from "../transport";

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

export function createAI(_t: Transport): AIApi {
  return {
    async generate(_options) {
      throw new Error(
        "@cocommand/api: AI.generate() is not yet implemented. " +
        "Requires POST /ai/generate Rust endpoint.",
      );
    },
  };
}
