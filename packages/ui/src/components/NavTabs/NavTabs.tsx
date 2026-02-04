import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./NavTabs.module.css";

type NavTabsProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function NavTabs({ children, className, ...props }: NavTabsProps) {
  return (
    <div className={cx(styles.tabs, className)} {...props}>
      {children}
    </div>
  );
}
