import "./App.css";
import { useEffect } from "react";
import { CommandBar } from "./components/CommandBar";
import { useServerStore } from "./state/server";
import { SettingsView } from "./views/settings/SettingsView";
import { useOnboardingStore } from "./state/onboarding";
import { UiKitView } from "./views/ui-kit/UiKitView";
import { OnboardingDemoView } from "./views/ui-kit/OnboardingDemoView";
import { SettingsDemoView } from "./views/ui-kit/SettingsDemoView";
import { ResponsesDemoView } from "./views/ui-kit/ResponsesDemoView";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import "@cocommand/ui";
import { Route, Routes, useLocation } from "react-router-dom";
import { AppContainer } from "./layout/AppContainer";
import { AppInit } from "./layout/AppInit";
import { OnboardingGate } from "./layout/OnboardingGate";

function App() {
  const { pathname } = useLocation();
  const serverInfo = useServerStore((state) => state.info);
  const isMainRoute = pathname === "/";
  const onboardingStatus = useOnboardingStore((state) => state.status);
  const onboardingLoaded = useOnboardingStore((state) => state.isLoaded);

  useEffect(() => {
    if (!isMainRoute) return;
    if (!serverInfo || !onboardingLoaded) return;
    const window = getCurrentWindow();
    const applyLayout = async () => {
      const size = onboardingStatus?.completed
        ? new LogicalSize(720, 240)
        : new LogicalSize(1200, 840);
      const minSize = onboardingStatus?.completed
        ? new LogicalSize(560, 200)
        : new LogicalSize(1040, 720);
      await window.setSize(size);
      await window.setMinSize(minSize);
    };
    void applyLayout();
  }, [isMainRoute, serverInfo, onboardingLoaded, onboardingStatus?.completed]);

  return (
    <AppContainer>
      <AppInit>
        <Routes>
          <Route path="/settings" element={<SettingsView />} />
          <Route path="/ui-kit" element={<UiKitView />} />
          <Route path="/ui-onboarding" element={<OnboardingDemoView />} />
          <Route path="/ui-settings" element={<SettingsDemoView />} />
          <Route path="/ui-responses" element={<ResponsesDemoView />} />
          <Route
            path="/"
            element={
              <OnboardingGate>
                <main className="container">
                  <CommandBar />
                </main>
              </OnboardingGate>
            }
          />
        </Routes>
      </AppInit>
    </AppContainer>
  );
}

export default App;
