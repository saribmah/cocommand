import "./App.css";
import { CommandView } from "./features/command/command.view";
import { NotesView } from "./features/notes/notes.view";
import { SettingsView } from "./features/settings/settings.view";
import { ExtensionWindowView } from "./features/command/components/ExtensionWindowView";
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
          <Route path="/notes" element={<NotesView />} />
          <Route path="/extension/:extensionId" element={<ExtensionWindowView />} />
          <Route path="/" element={<CommandView />} />
        </Routes>
      </AppInit>
    </AppContainer>
  );
}

export default App;
