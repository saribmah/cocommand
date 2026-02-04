import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ContentArea.module.css";

type ContentAreaProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function ContentArea({ children, className, ...props }: ContentAreaProps) {
  return (
    <div className={cx(styles.content, className)} {...props}>
      {children}
    </div>
  );
}
