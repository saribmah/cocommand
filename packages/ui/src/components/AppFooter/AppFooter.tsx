import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./AppFooter.module.css";

type AppFooterProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function AppFooter({ children, className, ...props }: AppFooterProps) {
  return (
    <footer className={cx(styles.footer, className)} {...props}>
      {children}
    </footer>
  );
}
