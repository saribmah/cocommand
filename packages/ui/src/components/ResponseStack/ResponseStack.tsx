import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ResponseStack.module.css";

type ResponseStackProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function ResponseStack({ children, className, ...props }: ResponseStackProps) {
  return (
    <div className={cx(styles.stack, className)} {...props}>
      {children}
    </div>
  );
}
