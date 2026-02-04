import "./App.css";
import { useEffect } from "react";
import { CommandBar } from "./components/CommandBar";
import { useServerStore } from "./state/server";
import { SettingsView } from "./views/settings/SettingsView";
import { useOnboardingStore } from "./state/onboarding";
import { OnboardingView } from "./views/onboarding/OnboardingView";
import { UiKitView } from "./views/ui-kit/UiKitView";
import { OnboardingDemoView } from "./views/ui-kit/OnboardingDemoView";
import { SettingsDemoView } from "./views/ui-kit/SettingsDemoView";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

function App() {
  const fetchServerInfo = useServerStore((state) => state.fetchInfo);
  const serverInfo = useServerStore((state) => state.info);
  const isSettings = window.location.pathname === "/settings";
  const isUiKit = window.location.pathname === "/ui-kit";
  const isUiOnboarding = window.location.pathname === "/ui-onboarding";
  const isUiSettings = window.location.pathname === "/ui-settings";
  const onboardingStatus = useOnboardingStore((state) => state.status);
  const onboardingLoaded = useOnboardingStore((state) => state.isLoaded);
  const onboardingError = useOnboardingStore((state) => state.error);
  const fetchOnboarding = useOnboardingStore((state) => state.fetchStatus);

  useEffect(() => {
    fetchServerInfo();
  }, [fetchServerInfo]);

  useEffect(() => {
    if (!serverInfo || onboardingLoaded) return;
    fetchOnboarding();
  }, [serverInfo, onboardingLoaded, fetchOnboarding]);

  useEffect(() => {
    if (!serverInfo) return;
    fetchOnboarding();
  }, [serverInfo?.addr, fetchOnboarding]);

  useEffect(() => {
    if (isSettings || isUiKit || isUiOnboarding || isUiSettings) return;
    if (!serverInfo || !onboardingLoaded) return;
    const window = getCurrentWindow();
    const applyLayout = async () => {
      const size = onboardingStatus?.completed
        ? new LogicalSize(720, 240)
        : new LogicalSize(1100, 720);
      const minSize = onboardingStatus?.completed
        ? new LogicalSize(560, 200)
        : new LogicalSize(920, 600);
      await window.setSize(size);
      await window.setMinSize(minSize);
    };
    void applyLayout();
  }, [isSettings, serverInfo, onboardingLoaded, onboardingStatus?.completed]);

  if (isSettings) {
    return <SettingsView />;
  }

  if (isUiKit) {
    return <UiKitView />;
  }

  if (isUiOnboarding) {
    return <OnboardingDemoView />;
  }

  if (isUiSettings) {
    return <SettingsDemoView />;
  }

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
    return <OnboardingView />;
  }

  return (
    <main className="container">
      <CommandBar />
    </main>
  );
}

export default App;
