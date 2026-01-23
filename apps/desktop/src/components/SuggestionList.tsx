interface Suggestion {
  app_id: string;
  score: number;
  explanation: string;
}

interface SuggestionListProps {
  suggestions: Suggestion[];
  selectedIndex: number;
}

export function SuggestionList({ suggestions, selectedIndex }: SuggestionListProps) {
  if (suggestions.length === 0) return null;

  return (
    <ul className="suggestion-list">
      {suggestions.map((candidate, i) => (
        <li
          key={candidate.app_id}
          className={`suggestion-item ${i === selectedIndex ? "selected" : ""}`}
        >
          <span className="suggestion-app">{candidate.app_id}</span>
          <span className="suggestion-explanation">{candidate.explanation}</span>
          <span className="suggestion-score">{Math.round(candidate.score)}</span>
        </li>
      ))}
    </ul>
  );
}
