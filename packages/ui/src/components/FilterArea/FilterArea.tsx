import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./FilterArea.module.css";

type FilterAreaProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function FilterArea({ children, className, ...props }: FilterAreaProps) {
  return (
    <div className={cx(styles.filter, className)} {...props}>
      {children}
    </div>
  );
}
