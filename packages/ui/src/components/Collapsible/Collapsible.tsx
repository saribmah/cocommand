import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./Collapsible.module.css";

type CollapsibleProps = HTMLAttributes<HTMLDivElement> & {
  label: string;
  open?: boolean;
  children: ReactNode;
};

export function Collapsible({ label, open = false, children, className, ...props }: CollapsibleProps) {
  return (
    <details className={cx(styles.details, className)} open={open} {...props}>
      <summary className={styles.summary}>{label}</summary>
      <div className={styles.content}>{children}</div>
    </details>
  );
}
