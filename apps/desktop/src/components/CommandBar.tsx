import { useRef, useEffect, type KeyboardEvent } from "react";
import { useCommandBar } from "../state/commandbar";
import { useServerStore } from "../state/server";
import { ResultCard } from "./ResultCard";
import { ConfirmPanel } from "./ConfirmPanel";
import "../styles/commandbar.css";

export function CommandBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const {
    input,
    isSubmitting,
    results,
    pendingConfirmation,
    followUpActive,
    setInput,
    submit,
    dismiss,
    dismissResult,
    confirmPending,
    cancelPending,
  } = useCommandBar();
  const serverInfo = useServerStore((state) => state.getInfo());

  useEffect(() => {
    inputRef.current?.focus();
  }, [results]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    switch (e.key) {
      case "Enter":
        e.preventDefault();
        submit();
        break;
      case "Escape":
        e.preventDefault();
        dismiss();
        break;
    }
  };

  return (
    <div className="command-bar">
      <div className="command-input-wrapper">
        {followUpActive && (
          <span className="follow-up-badge">Follow-up</span>
        )}
        <input
          ref={inputRef}
          className="command-input"
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={followUpActive ? "Refine the previous result\u2026" : "How can I help..."}
          disabled={isSubmitting || !!pendingConfirmation}
          spellCheck={false}
          autoComplete="off"
        />
        <span
          className="server-status-badge"
          data-status={serverInfo ? "online" : "offline"}
        >
          {serverInfo ? "Server online" : "Server offline"}
        </span>
      </div>
      <div className="command-results">
        {pendingConfirmation && (
          <ConfirmPanel
            confirmation={pendingConfirmation}
            onConfirm={confirmPending}
            onCancel={cancelPending}
          />
        )}
        <ResultCard results={results} onDismiss={dismissResult} />
      </div>
    </div>
  );
}
