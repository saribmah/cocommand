import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./Badge.module.css";

type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  tone?: "neutral" | "success" | "warn" | "error";
};

export function Badge({ tone = "neutral", className, children, ...props }: BadgeProps) {
  return (
    <span className={cx(styles.badge, styles[tone], className)} {...props}>
      {children}
    </span>
  );
}
