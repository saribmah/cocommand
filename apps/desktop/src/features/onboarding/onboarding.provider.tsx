import { AppPanel, ButtonPrimary, Text } from "@cocommand/ui";
import { type PropsWithChildren, useEffect } from "react";
import { useWorkspaceContext } from "../workspace/workspace.context";
import { OnboardingView } from "./onboarding.view";

type OnboardingProviderProps = PropsWithChildren;

export function OnboardingProvider({ children }: OnboardingProviderProps) {
  const config = useWorkspaceContext((state) => state.config);
  const isLoaded = useWorkspaceContext((state) => state.isLoaded);
  const error = useWorkspaceContext((state) => state.error);
  const fetchConfig = useWorkspaceContext((state) => state.fetchConfig);

  useEffect(() => {
    if (isLoaded) return;
    void fetchConfig();
  }, [isLoaded, fetchConfig]);

  console.log({config, isLoaded, error});

  if (error) {
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

  if (!isLoaded || !config) {
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
    return <OnboardingView />;
  }

  return <>{children}</>;
}
