interface SlashCommand {
  id: string;
  name: string;
  description: string;
}

interface SlashCommandPickerProps {
  commands: SlashCommand[];
  selectedIndex: number;
  onSelect: (command: SlashCommand) => void;
}

export function SlashCommandPicker({
  commands,
  selectedIndex,
  onSelect,
}: SlashCommandPickerProps) {
  if (commands.length === 0) return null;

  return (
    <ul className="suggestion-list">
      {commands.map((command, index) => (
        <li
          key={command.id}
          className={`suggestion-item ${index === selectedIndex ? "selected" : ""}`}
          onMouseDown={(event) => {
            event.preventDefault();
            onSelect(command);
          }}
        >
          <span className="suggestion-app">/{command.id}</span>
          <span className="suggestion-explanation">{command.description}</span>
        </li>
      ))}
    </ul>
  );
}
