import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./AppContent.module.css";

type AppContentProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function AppContent({ children, className, ...props }: AppContentProps) {
  return (
    <div className={cx(styles.content, className)} {...props}>
      {children}
    </div>
  );
}
