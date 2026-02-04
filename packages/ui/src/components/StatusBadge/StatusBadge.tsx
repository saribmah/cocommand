import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./StatusBadge.module.css";

type StatusBadgeProps = HTMLAttributes<HTMLSpanElement> & {
  status: "good" | "warn" | "neutral";
  label: string;
};

export function StatusBadge({ status, label, className, ...props }: StatusBadgeProps) {
  return (
    <span className={cx(styles.badge, styles[status], className)} {...props}>
      <Text as="span" size="xs" tone="secondary">
        {label}
      </Text>
    </span>
  );
}
