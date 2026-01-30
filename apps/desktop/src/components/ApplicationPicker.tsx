import type { ApplicationInfo } from "../types/application";

interface ApplicationPickerProps {
  applications: ApplicationInfo[];
  selectedIndex: number;
  onSelect: (app: ApplicationInfo) => void;
}

export function ApplicationPicker({
  applications,
  selectedIndex,
  onSelect,
}: ApplicationPickerProps) {
  if (applications.length === 0) return null;

  return (
    <ul className="suggestion-list">
      {applications.map((app, index) => (
        <li
          key={app.id}
          className={`suggestion-item ${index === selectedIndex ? "selected" : ""}`}
          onMouseDown={(event) => {
            event.preventDefault();
            onSelect(app);
          }}
        >
          <span className="suggestion-app">{app.name}</span>
          <span className="suggestion-explanation">{app.kind} • {app.id}</span>
        </li>
      ))}
    </ul>
  );
}
