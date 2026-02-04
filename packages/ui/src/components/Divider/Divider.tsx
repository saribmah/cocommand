import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./Divider.module.css";

type DividerProps = HTMLAttributes<HTMLDivElement> & {
  orientation?: "horizontal" | "vertical";
};

export function Divider({
  orientation = "horizontal",
  className,
  ...props
}: DividerProps) {
  return (
    <div
      role="separator"
      aria-orientation={orientation}
      className={cx(styles.divider, styles[orientation], className)}
      {...props}
    />
  );
}
