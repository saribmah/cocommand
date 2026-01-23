import { invoke } from "@tauri-apps/api/core";

export async function hideWindow(): Promise<void> {
  return invoke("hide_window");
}
