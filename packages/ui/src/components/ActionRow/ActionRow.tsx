import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ActionRow.module.css";

type ActionRowProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function ActionRow({ children, className, ...props }: ActionRowProps) {
  return (
    <div className={cx(styles.row, className)} {...props}>
      {children}
    </div>
  );
}
