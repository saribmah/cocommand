// @ts-check
import { invoke } from "@tauri-apps/api/core";

/**
 * Hide the main window.
 * @returns {Promise<void>}
 */
export async function hideWindow() {
  return invoke("hide_window");
}
