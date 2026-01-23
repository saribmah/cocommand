import { useEffect, useRef } from "react";
import type { ConfirmationResult } from "../types/core";
import { MarkdownView } from "./MarkdownView";

interface ConfirmPanelProps {
  confirmation: ConfirmationResult;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmPanel({ confirmation, onConfirm, onCancel }: ConfirmPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Enter") {
        e.preventDefault();
        onConfirm();
      } else if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onConfirm, onCancel]);

  return (
    <div className="confirm-panel" ref={panelRef}>
      <div className="confirm-panel-header">
        <span className="confirm-panel-title">{confirmation.title}</span>
      </div>
      <div className="confirm-panel-body">
        <MarkdownView content={confirmation.body} />
      </div>
      <div className="confirm-panel-actions">
        <button className="confirm-panel-btn confirm-panel-btn--cancel" onClick={onCancel}>
          Cancel
        </button>
        <button className="confirm-panel-btn confirm-panel-btn--confirm" onClick={onConfirm}>
          Confirm
        </button>
      </div>
      <div className="confirm-panel-hint">
        Enter to confirm &middot; Esc to cancel
      </div>
    </div>
  );
}
