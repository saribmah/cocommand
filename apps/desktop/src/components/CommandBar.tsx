import { useRef, useEffect, type KeyboardEvent } from "react";
import { useCommandBar } from "../state/commandbar";
import { SuggestionList } from "./SuggestionList";
import { ResultCard } from "./ResultCard";
import { ConfirmPanel } from "./ConfirmPanel";
import "../styles/commandbar.css";

export function CommandBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const {
    input,
    suggestions,
    selectedIndex,
    clarification,
    isSubmitting,
    results,
    pendingConfirmation,
    setInput,
    submit,
    navigateUp,
    navigateDown,
    dismiss,
    dismissResult,
    confirmPending,
    cancelPending,
  } = useCommandBar();

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
      case "ArrowUp":
        e.preventDefault();
        navigateUp();
        break;
      case "ArrowDown":
        e.preventDefault();
        navigateDown();
        break;
    }
  };

  return (
    <div className="command-bar">
      <div className="command-input-wrapper">
        <input
          ref={inputRef}
          className="command-input"
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a command..."
          disabled={isSubmitting || !!pendingConfirmation}
          spellCheck={false}
          autoComplete="off"
        />
      </div>
      {clarification && <div className="command-clarification">{clarification}</div>}
      <SuggestionList suggestions={suggestions} selectedIndex={selectedIndex} />
      {pendingConfirmation && (
        <ConfirmPanel
          confirmation={pendingConfirmation}
          onConfirm={confirmPending}
          onCancel={cancelPending}
        />
      )}
      <ResultCard results={results} onDismiss={dismissResult} />
    </div>
  );
}
