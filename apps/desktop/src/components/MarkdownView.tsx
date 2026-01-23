interface MarkdownViewProps {
  content: string;
}

export function MarkdownView({ content }: MarkdownViewProps) {
  const lines = content.split("\n");

  return (
    <div className="markdown-view">
      {lines.map((line, i) => (
        <p key={i} dangerouslySetInnerHTML={{ __html: renderInline(line) }} />
      ))}
    </div>
  );
}

function renderInline(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\*([^*]+)\*/g, "<em>$1</em>");
}
