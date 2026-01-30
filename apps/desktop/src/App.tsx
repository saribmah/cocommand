import "./App.css";
import { useEffect } from "react";
import { CommandBar } from "./components/CommandBar";
import { useServerStore } from "./state/server";
import { SettingsView } from "./views/settings/SettingsView";

function App() {
  const fetchServerInfo = useServerStore((state) => state.fetchInfo);
  const isSettings = window.location.pathname === "/settings";

  useEffect(() => {
    fetchServerInfo();
  }, [fetchServerInfo]);

  if (isSettings) {
    return <SettingsView />;
  }

  return (
    <main className="container">
      <CommandBar />
    </main>
  );
}

export default App;
