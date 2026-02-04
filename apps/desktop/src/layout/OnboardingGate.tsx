import type { ReactNode } from "react";
import { useEffect } from "react";
import { useOnboardingStore } from "../state/onboarding";
import { useServerStore } from "../state/server";
import { OnboardingRedesignView } from "../views/onboarding/OnboardingRedesignView";

interface OnboardingGateProps {
  children: ReactNode;
}

export function OnboardingGate({ children }: OnboardingGateProps) {
  const serverInfo = useServerStore((state) => state.info);
  const onboardingStatus = useOnboardingStore((state) => state.status);
  const onboardingLoaded = useOnboardingStore((state) => state.isLoaded);
  const onboardingError = useOnboardingStore((state) => state.error);
  const fetchOnboarding = useOnboardingStore((state) => state.fetchStatus);

  useEffect(() => {
    if (!serverInfo || onboardingLoaded) return;
    fetchOnboarding();
  }, [serverInfo, onboardingLoaded, fetchOnboarding]);

  useEffect(() => {
    if (!serverInfo) return;
    fetchOnboarding();
  }, [serverInfo?.addr, fetchOnboarding]);

  if (!serverInfo || !onboardingLoaded) {
    return (
      <main className="container">
        <div className="app-loading">Loading Cocommand...</div>
      </main>
    );
  }

  if (onboardingError) {
    return (
      <main className="container">
        <div className="app-loading">
          <p>Failed to load onboarding status.</p>
          <button type="button" onClick={() => fetchOnboarding()}>
            Retry
          </button>
        </div>
      </main>
    );
  }

  if (!onboardingStatus?.completed) {
    return <OnboardingRedesignView />;
  }

  return <>{children}</>;
}
