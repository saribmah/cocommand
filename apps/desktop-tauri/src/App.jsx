import { useState } from "react";
import "./App.css";
import { executeCommand } from "./lib/ipc";

function App() {
  const [result, setResult] = useState("");
  const [input, setInput] = useState("");

  async function submitCommand() {
    try {
      const response = await executeCommand(input);
      setResult(response.output);
    } catch (error) {
      setResult(`Error: ${error}`);
    }
  }

  return (
    <main className="container">
      <div className="hero">
        <div className="badge">cocommand.ai</div>
        <h1>Command bar, powered by intent.</h1>
        <p>Type a command and press Enter. This is a minimal IPC smoke test.</p>
      </div>

      <form
        className="command"
        onSubmit={(e) => {
          e.preventDefault();
          submitCommand();
        }}
      >
        <input
          id="command-input"
          value={input}
          onChange={(e) => setInput(e.currentTarget.value)}
          placeholder="Try: Move the file I just downloaded to Projects"
        />
        <button type="submit">Run</button>
      </form>
      <div className="result">
        <span>Result</span>
        <p>{result}</p>
      </div>
    </main>
  );
}

export default App;
