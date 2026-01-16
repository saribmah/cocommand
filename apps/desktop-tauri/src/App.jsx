import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import "./App.css";
import { executeCommand } from "./lib/ipc";

const WORKFLOWS = [
  {
    id: "move-downloads",
    name: "Organize downloads",
    prompt: "Move the file I just downloaded to Projects",
  },
  {
    id: "quick-note",
    name: "Quick note",
    prompt: "Quick Note: Draft Q3 strategy outline",
  },
  {
    id: "reply-message",
    name: "Reply to message",
    prompt: "Craft a professional response to the message I just received",
  },
  {
    id: "calendar-today",
    name: "Today's calendar",
    prompt: "What meetings are on my calendar today?",
  },
  {
    id: "reminder",
    name: "Set reminder",
    prompt: "Remind me about the xyz meeting in 15 minutes",
  },
];

function App() {
  const [result, setResult] = useState("");
  const [input, setInput] = useState("");
  const [history, setHistory] = useState([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [showWorkflows, setShowWorkflows] = useState(false);
  const [selectedWorkflowIndex, setSelectedWorkflowIndex] = useState(0);
  const draftInputRef = useRef("");
  const resizeFrameRef = useRef(0);
  const lastHeightRef = useRef(140);

  async function submitCommand() {
    try {
      const trimmed = input.trim();
      if (!trimmed) {
        setResult("Type a command to get started.");
        return;
      }
      const response = await executeCommand(trimmed);
      setHistory((prev) => [...prev, trimmed].slice(-20));
      setHistoryIndex(-1);
      setShowWorkflows(false);
      setInput("");
      setResult(response.output);
    } catch (error) {
      setResult(`Error: ${error}`);
    }
  }

  const filteredWorkflows = useMemo(() => {
    const query = input.trim().toLowerCase();
    if (!query) {
      return WORKFLOWS;
    }
    return WORKFLOWS.filter((workflow) =>
      workflow.name.toLowerCase().includes(query)
    );
  }, [input]);

  useEffect(() => {
    if (selectedWorkflowIndex > filteredWorkflows.length - 1) {
      setSelectedWorkflowIndex(0);
    }
  }, [filteredWorkflows, selectedWorkflowIndex]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const baseHeight = 140;
    const panelHeight = showWorkflows ? 220 : 0;
    const historyHeight = history.length > 0 ? 64 : 0;
    const resultHeight = result ? 110 : 0;
    const nextHeight = baseHeight + panelHeight + historyHeight + resultHeight;
    const windowHandle = getCurrentWindow();
    const startHeight = lastHeightRef.current;
    const targetHeight = nextHeight;
    const startTime = performance.now();
    const duration = 180;

    cancelAnimationFrame(resizeFrameRef.current);

    const step = (now) => {
      const elapsed = Math.min(now - startTime, duration);
      const progress = elapsed / duration;
      const eased = 1 - Math.pow(1 - progress, 3);
      const currentHeight = Math.round(
        startHeight + (targetHeight - startHeight) * eased
      );
      lastHeightRef.current = currentHeight;
      windowHandle
        .setSize(new LogicalSize(720, currentHeight))
        .catch(() => {});
      if (elapsed < duration) {
        resizeFrameRef.current = requestAnimationFrame(step);
      }
    };

    resizeFrameRef.current = requestAnimationFrame(step);
    return () => cancelAnimationFrame(resizeFrameRef.current);
  }, [showWorkflows, history.length, result]);

  function handleHistoryNavigation(direction) {
    if (history.length === 0) return;
    if (direction === "up") {
      if (historyIndex === -1) {
        draftInputRef.current = input;
        const nextIndex = history.length - 1;
        setHistoryIndex(nextIndex);
        setInput(history[nextIndex]);
        return;
      }
      const nextIndex = Math.max(0, historyIndex - 1);
      setHistoryIndex(nextIndex);
      setInput(history[nextIndex]);
      return;
    }

    if (historyIndex === -1) return;
    if (historyIndex >= history.length - 1) {
      setHistoryIndex(-1);
      setInput(draftInputRef.current);
      return;
    }
    const nextIndex = historyIndex + 1;
    setHistoryIndex(nextIndex);
    setInput(history[nextIndex]);
  }

  function handleWorkflowSelection(index) {
    const workflow = filteredWorkflows[index];
    if (!workflow) return;
    setInput(workflow.prompt);
    setShowWorkflows(false);
  }

  return (
    <main className="container">
      <form
        className="command"
        onSubmit={(e) => {
          e.preventDefault();
          submitCommand();
        }}
      >
        <div className="command-badge">coco</div>
        <input
          id="command-input"
          value={input}
          onChange={(e) => setInput(e.currentTarget.value)}
          onFocus={() => {
            if (showWorkflows && filteredWorkflows.length === 0) {
              setShowWorkflows(false);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Tab") {
              event.preventDefault();
              if (!showWorkflows) {
                setShowWorkflows(true);
                setSelectedWorkflowIndex(0);
                return;
              }
              const nextIndex =
                filteredWorkflows.length === 0
                  ? 0
                  : (selectedWorkflowIndex + 1) % filteredWorkflows.length;
              setSelectedWorkflowIndex(nextIndex);
              return;
            }

            if (event.key === "Escape") {
              if (showWorkflows) {
                setShowWorkflows(false);
                return;
              }
              setInput("");
              return;
            }

            if (showWorkflows && event.key === "ArrowDown") {
              event.preventDefault();
              const nextIndex = Math.min(
                filteredWorkflows.length - 1,
                selectedWorkflowIndex + 1
              );
              setSelectedWorkflowIndex(Math.max(0, nextIndex));
              return;
            }

            if (showWorkflows && event.key === "ArrowUp") {
              event.preventDefault();
              const nextIndex = Math.max(0, selectedWorkflowIndex - 1);
              setSelectedWorkflowIndex(nextIndex);
              return;
            }

            if (showWorkflows && event.key === "Enter") {
              event.preventDefault();
              if (filteredWorkflows.length > 0) {
                handleWorkflowSelection(selectedWorkflowIndex);
              } else {
                setShowWorkflows(false);
                submitCommand();
              }
              return;
            }

            if (!showWorkflows && event.key === "ArrowUp") {
              event.preventDefault();
              handleHistoryNavigation("up");
              return;
            }

            if (!showWorkflows && event.key === "ArrowDown") {
              event.preventDefault();
              handleHistoryNavigation("down");
            }
          }}
          placeholder="Ask coco to do something..."
        />
        <button type="submit">Run</button>
      </form>

      {showWorkflows && (
        <div className="panel">
          <div className="panel-header">
            <span>Workflows</span>
            <span className="panel-hint">Tab to cycle • Enter to apply</span>
          </div>
          <ul className="panel-list">
            {filteredWorkflows.length === 0 && (
              <li className="panel-empty">No workflows match that search.</li>
            )}
            {filteredWorkflows.map((workflow, index) => (
              <li key={workflow.id}>
                <button
                  type="button"
                  className={
                    index === selectedWorkflowIndex ? "active" : undefined
                  }
                  onMouseDown={(event) => {
                    event.preventDefault();
                    handleWorkflowSelection(index);
                  }}
                >
                  <span>{workflow.name}</span>
                  <small>{workflow.prompt}</small>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      {history.length > 0 && (
        <div className="history">
          <span>Recent</span>
          <div className="history-row">
            {history.slice(-3).map((item, index) => (
              <button
                key={`${item}-${index}`}
                type="button"
                onClick={() => setInput(item)}
              >
                {item}
              </button>
            ))}
          </div>
        </div>
      )}
      {result && (
        <div className="result">
          <span>Result</span>
          <p>{result}</p>
        </div>
      )}
    </main>
  );
}

export default App;
