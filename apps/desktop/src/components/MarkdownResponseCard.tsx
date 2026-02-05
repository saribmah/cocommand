import type { HTMLAttributes } from "react";
import { ResponseBlock, ResponseHeader } from "@cocommand/ui";
import styles from "./MarkdownResponseCard.module.css";

interface MarkdownResponseCardProps extends HTMLAttributes<HTMLDivElement> {
  label?: string;
  body: string;
}

export function MarkdownResponseCard({
  label = "Assistant",
  body,
  className,
  ...props
}: MarkdownResponseCardProps) {
  const classes = [styles.card, className].filter(Boolean).join(" ");

  return (
    <ResponseBlock className={classes} {...props}>
      <ResponseHeader label={label} />
      <div className={styles.body}>
        <MarkdownView content={body} />
      </div>
    </ResponseBlock>
  );
}

interface MarkdownViewProps {
  content: string;
}

function MarkdownView({ content }: MarkdownViewProps) {
  const lines = content.split("\n");

  return (
    <div className={styles.markdown}>
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
