import type { CoreResult } from "../types/core";
import { MarkdownView } from "./MarkdownView";

interface ResultCardProps {
  results: CoreResult[];
  onAction?: (actionId: string) => void;
  onDismiss?: (index: number) => void;
}

export function ResultCard({ results, onAction, onDismiss }: ResultCardProps) {
  const visible = results.slice(-1);

  if (visible.length === 0) return null;

  return (
    <div className="result-card-stack">
      {visible.map((result, i) => (
        <div key={i} className={`result-card result-card--${result.type}`}>
          <div className="result-card-header">
            <span className="result-card-title">{result.title}</span>
            {onDismiss && (
              <button
                className="result-card-dismiss"
                onClick={() => onDismiss(0)}
                aria-label="Dismiss"
              >
                &times;
              </button>
            )}
          </div>
          <div className="result-card-body">
            <MarkdownView content={result.body} />
          </div>
          {result.type === "artifact" && result.actions.length > 0 && (
            <div className="result-card-actions">
              {result.actions.map((action) => (
                <button
                  key={action.id}
                  className="result-card-action-btn"
                  onClick={() => onAction?.(action.id)}
                >
                  {action.label}
                </button>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
