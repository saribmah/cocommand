import { create } from "zustand";
import type { OnboardingStatus, UpdateOnboardingPayload } from "../types/onboarding";
import { useServerStore } from "./server";

interface OnboardingState {
  status: OnboardingStatus | null;
  isLoaded: boolean;
  error: string | null;
  fetchStatus: () => Promise<void>;
  updateStatus: (payload: UpdateOnboardingPayload) => Promise<OnboardingStatus>;
}

function buildServerUrl(addr: string, path: string): string {
  const prefix = addr.startsWith("http") ? addr : `http://${addr}`;
  return `${prefix}${path}`;
}

export const useOnboardingStore = create<OnboardingState>((set) => ({
  status: null,
  isLoaded: false,
  error: null,
  fetchStatus: async () => {
    const server = useServerStore.getState().info;
    if (!server) {
      set({ status: null, isLoaded: false, error: null });
      return;
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/onboarding");
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Server error (${response.status})`);
      }
      const data = (await response.json()) as OnboardingStatus;
      set({ status: data, isLoaded: true, error: null });
    } catch (err) {
      set({ status: null, isLoaded: true, error: String(err) });
    }
  },
  updateStatus: async (payload) => {
    const server = useServerStore.getState().info;
    if (!server) {
      throw new Error("Server unavailable");
    }
    const url = buildServerUrl(server.addr, "/workspace/settings/onboarding");
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(errorText || `Server error (${response.status})`);
    }
    const data = (await response.json()) as OnboardingStatus;
    set({ status: data, isLoaded: true, error: null });
    return data;
  },
}));
