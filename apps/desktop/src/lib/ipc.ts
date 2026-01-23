import { invoke } from "@tauri-apps/api/core";
import type { CoreResult, ArtifactAction } from "../types/core";

export interface RoutedCandidate {
  app_id: string;
  score: number;
  explanation: string;
}

export type CoreResponse =
  | { type: "Routed"; candidates: RoutedCandidate[]; follow_up_active: boolean }
  | { type: "ClarificationNeeded"; message: string }
  | { type: "Artifact"; title: string; body: string; actions: ArtifactAction[] }
  | { type: "Preview"; title: string; body: string }
  | { type: "Confirmation"; title: string; body: string; confirmation_id: string }
  | { type: "Error"; title: string; body: string };

export function normalizeResponse(response: CoreResponse): CoreResult | null {
  switch (response.type) {
    case "Artifact":
      return { type: "artifact", title: response.title, body: response.body, actions: response.actions };
    case "Preview":
      return { type: "preview", title: response.title, body: response.body };
    case "Confirmation":
      return { type: "confirmation", title: response.title, body: response.body, confirmation_id: response.confirmation_id };
    case "Error":
      return { type: "error", title: response.title, body: response.body };
    case "Routed":
    case "ClarificationNeeded":
      return null;
  }
}

export async function submitCommand(text: string): Promise<CoreResponse> {
  return invoke("submit_command", { text });
}

export async function hideWindow(): Promise<void> {
  return invoke("hide_window");
}
