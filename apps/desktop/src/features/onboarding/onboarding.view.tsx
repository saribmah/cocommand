import { AppPanel, Text } from "@cocommand/ui";

export function OnboardingView() {
  return (
    <AppPanel style={{ minHeight: 420, maxWidth: 960 }}>
      <Text as="div" size="lg" weight="semibold">
        Onboarding view is not implemented yet.
      </Text>
      <Text as="div" size="sm" tone="secondary">
        Next step: migrate onboarding UI into the new feature provider flow.
      </Text>
    </AppPanel>
  );
}
