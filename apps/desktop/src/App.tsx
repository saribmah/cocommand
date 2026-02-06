import "./App.css";
import {
  OnboardingDemoView,
  ResponsesDemoView,
  SettingsDemoView,
  UiKitView,
} from "@cocommand/demo";
import { CommandView } from "./features/command/command.view";
import { SettingsView } from "./features/settings/settings.view";
import "@cocommand/ui";
import { Route, Routes } from "react-router-dom";
import { AppContainer } from "./layout/AppContainer";
import { AppInit } from "./layout/AppInit";

function App() {
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
              <main className="container">
                <CommandView />
              </main>
            }
          />
        </Routes>
      </AppInit>
    </AppContainer>
  );
}

export default App;
