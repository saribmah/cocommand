import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Collapsible } from "../Collapsible/Collapsible";
import { CodeBlock } from "../CodeBlock/CodeBlock";
import { ResponseBlock } from "../ResponseBlock/ResponseBlock";
import { ResponseHeader } from "../ResponseHeader/ResponseHeader";
import styles from "./ReasoningCard.module.css";

type ReasoningCardProps = HTMLAttributes<HTMLDivElement> & {
  label?: string;
  reasoning: string;
  defaultOpen?: boolean;
};

export function ReasoningCard({
  label = "Reasoning",
  reasoning,
  defaultOpen = false,
  className,
  ...props
}: ReasoningCardProps) {
  return (
    <ResponseBlock className={cx(styles.card, className)} {...props}>
      <ResponseHeader label={label} />
      <Collapsible label="Show reasoning" open={defaultOpen}>
        <CodeBlock code={reasoning} />
      </Collapsible>
    </ResponseBlock>
  );
}
