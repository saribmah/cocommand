import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./Pill.module.css";

type PillProps = HTMLAttributes<HTMLSpanElement> & {
  size?: "sm" | "md";
};

export function Pill({ size = "sm", className, ...props }: PillProps) {
  return <span className={cx(styles.pill, styles[`size-${size}`], className)} {...props} />;
}
