import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./FooterArea.module.css";

type FooterAreaProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function FooterArea({ children, className, ...props }: FooterAreaProps) {
  return (
    <div className={cx(styles.footer, className)} {...props}>
      {children}
    </div>
  );
}
