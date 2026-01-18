import { useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import "./App.css";
import { hideWindow } from "./lib/ipc";

function App() {
  const [input, setInput] = useState("");
  const [result, setResult] = useState("");
  const [backendStatus, setBackendStatus] = useState("checking");
  const resizeFrameRef = useRef(0);
  const lastHeightRef = useRef(140);
  const inputRef = useRef(null);

  async function submitCommand() {
    try {
      const trimmed = input.trim();
      if (!trimmed) {
        setResult("Type a command to get started.");
        return;
      }
      const response = await fetch("http://127.0.0.1:4840/command", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: trimmed }),
      }).then((res) => res.json());

      if (response.status !== "ok") {
        setResult(response.message ?? "Unable to run the command.");
        return;
      }

      setInput("");
      setResult(response.result?.message ?? "Command executed.");
    } catch (error) {
      setResult(`Error: ${error}`);
    }
  }

  useEffect(() => {
    let active = true;
    const check = () => {
      fetch("http://127.0.0.1:4840/health")
        .then((response) => response.json())
        .then((data) => {
          if (!active) return;
          const next = data?.status === "ok" ? "ok" : "error";
          setBackendStatus(next);
          if (next === "ok") {
            clearInterval(timer);
          }
        })
        .catch(() => {
          if (!active) return;
          setBackendStatus("error");
        });
    };
    const timer = setInterval(check, 800);
    check();
    return () => {
      active = false;
      clearInterval(timer);
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
    const resultHeight = result ? 90 : 0;
    const nextHeight = baseHeight + resultHeight;
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
  }, [result]);

  return (
    <main className={result ? "container stacked" : "container"}>
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
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              if (input.trim()) {
                setInput("");
                return;
              }
              hideWindow().catch(() => {});
            }
          }}
          placeholder="Ask coco to do something..."
        />
        <button type="submit">Run</button>
      </form>

      {result && (
        <div className="result">
          <span>Result</span>
          <p>{result}</p>
        </div>
      )}

      {backendStatus !== "checking" && (
        <div
          className={
            backendStatus === "ok"
              ? "backend-banner backend-banner-ok"
              : "backend-banner backend-banner-error"
          }
        >
          {backendStatus === "ok"
            ? "Backend connected"
            : "Backend not reachable. Ensure the server is running."}
        </div>
      )}
    </main>
  );
}

export default App;
