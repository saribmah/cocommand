import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./HeaderArea.module.css";

type HeaderAreaProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function HeaderArea({ children, className, ...props }: HeaderAreaProps) {
  return (
    <div className={cx(styles.header, className)} {...props}>
      {children}
    </div>
  );
}
