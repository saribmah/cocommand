import type { Transport } from "../transport";

export interface ToastOptions {
  message: string;
  type?: "info" | "success" | "warning" | "error";
  duration?: number;
}

export interface WindowManagementApi {
  resize(width: number, height: number): Promise<void>;
}

export function createWindowManagement(_t: Transport): {
  showToast: (options: ToastOptions) => Promise<void>;
  windowManagement: WindowManagementApi;
} {
  return {
    async showToast(_options) {
      throw new Error(
        "@cocommand/api: showToast() is not yet implemented. " +
        "Requires POST /ui/toast Rust endpoint and frontend toast component.",
      );
    },
    windowManagement: {
      async resize(_width, _height) {
        throw new Error(
          "@cocommand/api: WindowManagement.resize() is not yet implemented. " +
          "Requires Tauri window management endpoint.",
        );
      },
    },
  };
}
