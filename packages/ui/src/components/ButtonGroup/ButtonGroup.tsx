import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ButtonGroup.module.css";

type ButtonGroupProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function ButtonGroup({ children, className, ...props }: ButtonGroupProps) {
  return (
    <div className={cx(styles.group, className)} {...props}>
      {children}
    </div>
  );
}
