import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./HighlightGrid.module.css";

type HighlightGridProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function HighlightGrid({ children, className, ...props }: HighlightGridProps) {
  return (
    <div className={cx(styles.grid, className)} {...props}>
      {children}
    </div>
  );
}
