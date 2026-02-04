import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./FieldRow.module.css";

type FieldRowProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function FieldRow({ children, className, ...props }: FieldRowProps) {
  return (
    <div className={cx(styles.row, className)} {...props}>
      {children}
    </div>
  );
}
