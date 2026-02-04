import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ResponseBlock.module.css";

type ResponseBlockProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function ResponseBlock({ children, className, ...props }: ResponseBlockProps) {
  return (
    <section className={cx(styles.block, className)} {...props}>
      {children}
    </section>
  );
}
