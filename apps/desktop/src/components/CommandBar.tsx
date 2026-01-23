import { useRef, useEffect, type KeyboardEvent } from "react";
import { useCommandBar } from "../state/commandbar";
import { SuggestionList } from "./SuggestionList";
import "../styles/commandbar.css";

export function CommandBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const {
    input,
    suggestions,
    selectedIndex,
    clarification,
    isSubmitting,
    setInput,
    submit,
    navigateUp,
    navigateDown,
    dismiss,
  } = useCommandBar();

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

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
          disabled={isSubmitting}
          spellCheck={false}
          autoComplete="off"
        />
      </div>
      {clarification && <div className="command-clarification">{clarification}</div>}
      <SuggestionList suggestions={suggestions} selectedIndex={selectedIndex} />
    </div>
  );
}
