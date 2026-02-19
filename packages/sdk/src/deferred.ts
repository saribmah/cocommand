import { notImplemented } from "./errors";

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

export function createDeferredAI(): AIApi {
  return {
    async generate(_options) {
      notImplemented("ai.generate", {
        reason: "AI endpoint is intentionally deferred in this phase",
        migration: "Use sdk.tools.invoke(...) against an extension-provided AI tool until native SDK AI APIs ship.",
      });
    },
  };
}

export interface LocalStorageApi {
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T): Promise<void>;
  delete(key: string): Promise<void>;
  keys(): Promise<string[]>;
}

export function createDeferredLocalStorage(extensionId: string): LocalStorageApi {
  return {
    async get<T>(_key: string): Promise<T | null> {
      notImplemented("localStorage.get", {
        extensionId,
        reason: "Extension storage routes are intentionally deferred in this phase",
        migration: "Persist data externally or via extension tools until extension storage routes are implemented.",
      });
    },
    async set<T>(_key: string, _value: T): Promise<void> {
      notImplemented("localStorage.set", {
        extensionId,
        reason: "Extension storage routes are intentionally deferred in this phase",
        migration: "Persist data externally or via extension tools until extension storage routes are implemented.",
      });
    },
    async delete(_key: string): Promise<void> {
      notImplemented("localStorage.delete", {
        extensionId,
        reason: "Extension storage routes are intentionally deferred in this phase",
        migration: "Persist data externally or via extension tools until extension storage routes are implemented.",
      });
    },
    async keys(): Promise<string[]> {
      notImplemented("localStorage.keys", {
        extensionId,
        reason: "Extension storage routes are intentionally deferred in this phase",
        migration: "Persist data externally or via extension tools until extension storage routes are implemented.",
      });
    },
  };
}

export interface ToastOptions {
  message: string;
  type?: "info" | "success" | "warning" | "error";
  duration?: number;
}

export interface WindowManagementApi {
  resize(width: number, height: number): Promise<void>;
}

export interface UiApi {
  showToast(options: ToastOptions): Promise<void>;
  windowManagement: WindowManagementApi;
}

export function createDeferredUiApi(): UiApi {
  return {
    async showToast(_options: ToastOptions): Promise<void> {
      notImplemented("ui.showToast", {
        reason: "Host window/toast bridge is intentionally deferred in this phase",
        migration: "Use existing host UI notifications until the sdk ui bridge is implemented.",
      });
    },
    windowManagement: {
      async resize(_width: number, _height: number): Promise<void> {
        notImplemented("ui.windowManagement.resize", {
          reason: "Host window/toast bridge is intentionally deferred in this phase",
          migration: "Use the current host window APIs until sdk window management is implemented.",
        });
      },
    },
  };
}
