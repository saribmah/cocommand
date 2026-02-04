import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./AppNav.module.css";

type AppNavProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function AppNav({ children, className, ...props }: AppNavProps) {
  return (
    <nav className={cx(styles.nav, className)} {...props}>
      {children}
    </nav>
  );
}
