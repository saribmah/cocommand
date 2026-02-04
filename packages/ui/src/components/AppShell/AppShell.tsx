import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./AppShell.module.css";

type AppShellProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function AppShell({ children, className, ...props }: AppShellProps) {
  return (
    <main className={cx(styles.shell, className)} {...props}>
      {children}
    </main>
  );
}
