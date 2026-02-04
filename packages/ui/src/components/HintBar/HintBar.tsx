import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./HintBar.module.css";

type HintBarProps = HTMLAttributes<HTMLDivElement> & {
  left?: ReactNode;
  right?: ReactNode;
};

export function HintBar({ left, right, className, ...props }: HintBarProps) {
  return (
    <div className={cx(styles.bar, className)} {...props}>
      <div className={styles.group}>{left}</div>
      <div className={cx(styles.group, styles.right)}>{right}</div>
    </div>
  );
}
