import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import "./App.css";
import { hideWindow, listCommands, listWorkflows, planCommand } from "./lib/ipc";

function App() {
  const [result, setResult] = useState("");
  const [plan, setPlan] = useState(null);
  const [input, setInput] = useState("");
  const [history, setHistory] = useState([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [showCommands, setShowCommands] = useState(false);
  const [selectedCommandIndex, setSelectedCommandIndex] = useState(0);
  const draftInputRef = useRef("");
  const resizeFrameRef = useRef(0);
  const lastHeightRef = useRef(140);
  const inputRef = useRef(null);
  const [commands, setCommands] = useState([]);
  const [commandErrors, setCommandErrors] = useState([]);
  const [workflows, setWorkflows] = useState([]);
  const [workflowErrors, setWorkflowErrors] = useState([]);
  const isStacked = showCommands || history.length > 0 || Boolean(result);

  async function submitCommand() {
    try {
      const trimmed = input.trim();
      if (!trimmed) {
        setPlan(null);
        setResult("Type a command to get started.");
        return;
      }
      const planResponse = await planCommand(trimmed);
      if (planResponse.status === "empty") {
        setPlan(null);
        setResult(planResponse.message ?? "Type a command to get started.");
        return;
      }
      if (planResponse.status !== "ok" || !planResponse.plan) {
        setPlan(null);
        setResult(planResponse.message ?? "Unable to plan the request.");
        return;
      }
      const confidence = Math.round(planResponse.plan.intent.confidence * 100);
      const steps = planResponse.plan.steps.map((step) => step.tool).join(", ");
      const summary = steps
        ? `Intent: ${planResponse.plan.intent.name} (${confidence}%). Steps: ${steps}.`
        : `Intent: ${planResponse.plan.intent.name} (${confidence}%). No steps planned.`;
      setHistory((prev) => [...prev, trimmed].slice(-20));
      setHistoryIndex(-1);
      setShowCommands(false);
      setInput("");
      setResult(summary);
      setPlan(planResponse.plan);
    } catch (error) {
      setPlan(null);
      setResult(`Error: ${error}`);
    }
  }

  const filteredCommands = useMemo(() => {
    const query = input.trim().toLowerCase();
    const combined = [...commands, ...workflows];
    if (!query) {
      return combined;
    }
    return combined.filter((command) =>
      command.name.toLowerCase().includes(query)
    );
  }, [input, commands, workflows]);

  useEffect(() => {
    if (selectedCommandIndex > filteredCommands.length - 1) {
      setSelectedCommandIndex(0);
    }
  }, [filteredCommands, selectedCommandIndex]);

  useEffect(() => {
    let cancelled = false;
    listCommands()
      .then((response) => {
        if (cancelled) return;
        const normalized = response.commands.map((command) => ({
          ...command,
          prompt: command.description ?? command.name,
        }));
        setCommands(normalized);
        setCommandErrors(response.errors ?? []);
      })
      .catch((error) => {
        if (cancelled) return;
        setCommandErrors([{ file: "backend", message: String(error) }]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    listWorkflows()
      .then((response) => {
        if (cancelled) return;
        const normalized = response.workflows.map((workflow) => ({
          ...workflow,
          prompt: workflow.description ?? workflow.name,
          isWorkflow: true,
        }));
        setWorkflows(normalized);
        setWorkflowErrors(response.errors ?? []);
      })
      .catch((error) => {
        if (cancelled) return;
        setWorkflowErrors([{ file: "backend", message: String(error) }]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const windowHandle = getCurrentWindow();
    let unlisten;
    windowHandle
      .onFocusChanged(({ payload: focused }) => {
        if (focused) {
          requestAnimationFrame(() => {
            inputRef.current?.focus();
            inputRef.current?.select();
          });
        } else {
          windowHandle.hide().catch(() => {});
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const baseHeight = 140;
    const panelHeight = showCommands ? 220 : 0;
    const historyHeight = history.length > 0 ? 64 : 0;
    const planHeight = plan ? 80 + plan.steps.length * 28 : 0;
    const resultHeight = result ? 90 : 0;
    const nextHeight =
      baseHeight + panelHeight + historyHeight + resultHeight + planHeight;
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
  }, [showCommands, history.length, result]);

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

  function handleCommandSelection(index) {
    const command = filteredCommands[index];
    if (!command) return;
    setInput(command.prompt ?? command.name);
    setShowCommands(false);
  }

  return (
    <main className={isStacked ? "container stacked" : "container"}>
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
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.currentTarget.value)}
          onFocus={() => {
            if (showCommands && filteredCommands.length === 0) {
              setShowCommands(false);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Tab") {
              event.preventDefault();
              if (!showCommands) {
                setShowCommands(true);
                setSelectedCommandIndex(0);
                return;
              }
              const nextIndex =
                filteredCommands.length === 0
                  ? 0
                  : (selectedCommandIndex + 1) % filteredCommands.length;
              setSelectedCommandIndex(nextIndex);
              return;
            }

            if (event.key === "Escape") {
              if (showCommands) {
                setShowCommands(false);
                return;
              }
              if (input.trim()) {
                setInput("");
                return;
              }
              hideWindow().catch(() => {});
              return;
            }

            if (showCommands && event.key === "ArrowDown") {
              event.preventDefault();
              const nextIndex = Math.min(
                filteredCommands.length - 1,
                selectedCommandIndex + 1
              );
              setSelectedCommandIndex(Math.max(0, nextIndex));
              return;
            }

            if (showCommands && event.key === "ArrowUp") {
              event.preventDefault();
              const nextIndex = Math.max(0, selectedCommandIndex - 1);
              setSelectedCommandIndex(nextIndex);
              return;
            }

            if (showCommands && event.key === "Enter") {
              event.preventDefault();
              if (filteredCommands.length > 0) {
                handleCommandSelection(selectedCommandIndex);
              } else {
                setShowCommands(false);
                submitCommand();
              }
              return;
            }

            if (!showCommands && event.key === "ArrowUp") {
              event.preventDefault();
              handleHistoryNavigation("up");
              return;
            }

            if (!showCommands && event.key === "ArrowDown") {
              event.preventDefault();
              handleHistoryNavigation("down");
            }
          }}
          placeholder="Ask coco to do something..."
        />
        <button type="submit">Run</button>
      </form>

      {showCommands && (
        <div className="panel">
          <div className="panel-header">
            <span>Commands</span>
            <span className="panel-hint">Tab to cycle • Enter to apply</span>
          </div>
          <ul className="panel-list">
            {commandErrors.length > 0 && (
              <li className="panel-error">
                {commandErrors[0].message}
              </li>
            )}
            {workflowErrors.length > 0 && (
              <li className="panel-error">
                {workflowErrors[0].message}
              </li>
            )}
            {filteredCommands.length === 0 && (
              <li className="panel-empty">No commands match that search.</li>
            )}
            {filteredCommands.map((command, index) => (
              <li key={command.id}>
                <button
                  type="button"
                  className={
                    index === selectedCommandIndex ? "active" : undefined
                  }
                  onMouseDown={(event) => {
                    event.preventDefault();
                    handleCommandSelection(index);
                  }}
                >
                  <span>{command.name}</span>
                  <small>
                    {command.prompt}
                    {command.isWorkflow ? " · workflow" : ""}
                  </small>
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
          {plan && (
            <div className="plan-details">
              <div className="plan-row">
                <span>Intent</span>
                <strong>{plan.intent.name}</strong>
              </div>
              <div className="plan-row">
                <span>Confidence</span>
                <strong>{Math.round(plan.intent.confidence * 100)}%</strong>
              </div>
              <div className="plan-steps">
                {plan.steps.length === 0 ? (
                  <span>No steps planned</span>
                ) : (
                  plan.steps.map((step) => (
                    <div key={step.id} className="plan-step">
                      <span>{step.tool}</span>
                      <small>{step.status}</small>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </main>
  );
}

export default App;
