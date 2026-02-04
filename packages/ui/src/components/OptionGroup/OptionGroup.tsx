import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./OptionGroup.module.css";

type OptionGroupProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function OptionGroup({ children, className, ...props }: OptionGroupProps) {
  return (
    <div className={cx(styles.group, className)} {...props}>
      {children}
    </div>
  );
}
