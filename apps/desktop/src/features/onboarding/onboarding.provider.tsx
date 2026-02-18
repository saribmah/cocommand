import { AppPanel, ButtonPrimary, Text } from "@cocommand/ui";
import { type PropsWithChildren, useEffect } from "react";
import { useStore } from "zustand";
import { useExtensionStore } from "../extension/extension.context";
import type { WorkspaceExtensionState } from "../workspace/workspace.extension-store";
import { WorkspaceView } from "../workspace/workspace.view";

type OnboardingProviderProps = PropsWithChildren;

export function OnboardingProvider({ children }: OnboardingProviderProps) {
  const store = useExtensionStore<WorkspaceExtensionState>("workspace");
  const config = useStore(store, (s) => s.config);
  const isLoading = useStore(store, (s) => s.isLoading);
  const error = useStore(store, (s) => s.error);
  const fetchConfig = useStore(store, (s) => s.fetchConfig);

  useEffect(() => {
    if (!config && !isLoading) void fetchConfig();
  }, [config, isLoading, fetchConfig]);

  if (error && !config) {
    return (
      <AppPanel style={{ minHeight: 360, maxWidth: 620 }}>
        <Text as="div" size="lg" weight="semibold">
          Failed to load workspace config
        </Text>
        <Text as="div" size="sm" tone="secondary">
          {error}
        </Text>
        <ButtonPrimary onClick={() => void fetchConfig()}>Retry</ButtonPrimary>
      </AppPanel>
    );
  }

  if (!config) {
    return (
      <AppPanel style={{ minHeight: 360, maxWidth: 620 }}>
        <Text as="div" size="lg" weight="semibold">
          Loading workspace...
        </Text>
        <Text as="div" size="sm" tone="secondary">
          Fetching workspace configuration.
        </Text>
      </AppPanel>
    );
  }

  if (!config.onboarding.completed) {
    return <WorkspaceView mode="inline" />;
  }

  return <>{children}</>;
}
