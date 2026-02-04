import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { ActionRow } from "../ActionRow/ActionRow";
import { Badge } from "../Badge/Badge";
import { Collapsible } from "../Collapsible/Collapsible";
import { CodeBlock } from "../CodeBlock/CodeBlock";
import { ResponseBlock } from "../ResponseBlock/ResponseBlock";
import { ResponseHeader } from "../ResponseHeader/ResponseHeader";
import styles from "./ToolCallCard.module.css";

type ToolCallState = "pending" | "running" | "success" | "error";

type ToolCallCardProps = HTMLAttributes<HTMLDivElement> & {
  toolName: string;
  toolId: string;
  state?: ToolCallState;
  params?: string;
  result?: string;
  errorMessage?: string;
  icon?: ReactNode;
};

export function ToolCallCard({
  toolName,
  toolId,
  state = "pending",
  params,
  result,
  errorMessage,
  icon,
  className,
  ...props
}: ToolCallCardProps) {
  const badgeTone =
    state === "success" ? "success" : state === "error" ? "error" : state === "running" ? "warn" : "neutral";

  return (
    <ResponseBlock className={cx(styles.card, className)} {...props}>
      <ResponseHeader
        icon={icon}
        label={toolName}
        meta={
          <ActionRow>
            <Badge tone={badgeTone}>{state}</Badge>
            <span className={styles.toolId}>#{toolId}</span>
          </ActionRow>
        }
      />

      {params ? (
        <Collapsible label="Parameters" open={state !== "pending" && state !== "running"}>
          <CodeBlock code={params} />
        </Collapsible>
      ) : null}

      {state === "success" && result ? (
        <div className={styles.section}>
          <div className={styles.sectionLabel}>Result</div>
          <CodeBlock code={result} />
        </div>
      ) : null}

      {state === "error" && errorMessage ? (
        <div className={styles.section}>
          <div className={styles.sectionLabel}>Error</div>
          <div className={styles.error}>{errorMessage}</div>
        </div>
      ) : null}
    </ResponseBlock>
  );
}
