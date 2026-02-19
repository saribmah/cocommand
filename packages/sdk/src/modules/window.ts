import type { Client } from "../client";

export interface ToastOptions {
  message: string;
  type?: "info" | "success" | "warning" | "error";
  duration?: number;
}

export interface WindowManagementApi {
  resize(width: number, height: number): Promise<void>;
}

export function createWindowManagement(_client: Client): {
  showToast: (options: ToastOptions) => Promise<void>;
  windowManagement: WindowManagementApi;
} {
  return {
    async showToast(_options) {
      throw new Error(
        "@cocommand/sdk: showToast() is not yet implemented. " +
        "Requires POST /ui/toast Rust endpoint and frontend toast component.",
      );
    },
    windowManagement: {
      async resize(_width, _height) {
        throw new Error(
          "@cocommand/sdk: WindowManagement.resize() is not yet implemented. " +
          "Requires Tauri window management endpoint.",
        );
      },
    },
  };
}
