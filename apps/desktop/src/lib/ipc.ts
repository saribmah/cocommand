import { invoke } from "@tauri-apps/api/core";

export interface RoutedCandidate {
  app_id: string;
  score: number;
  explanation: string;
}

export type CoreResponse =
  | { type: "Routed"; candidates: RoutedCandidate[]; follow_up_active: boolean }
  | { type: "ClarificationNeeded"; message: string };

export async function submitCommand(text: string): Promise<CoreResponse> {
  return invoke("submit_command", { text });
}

export async function hideWindow(): Promise<void> {
  return invoke("hide_window");
}
