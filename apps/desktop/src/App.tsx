import "./App.css";
import { useEffect } from "react";
import { CommandBar } from "./components/CommandBar";
import { useServerStore } from "./state/server";

function App() {
  const fetchServerInfo = useServerStore((state) => state.fetchInfo);

  useEffect(() => {
    fetchServerInfo();
  }, [fetchServerInfo]);

  return (
    <main className="container">
      <CommandBar />
    </main>
  );
}

export default App;
