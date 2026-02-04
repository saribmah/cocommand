import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./InfoCard.module.css";

type InfoCardProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function InfoCard({ children, className, ...props }: InfoCardProps) {
  return (
    <div className={cx(styles.card, className)} {...props}>
      {children}
    </div>
  );
}
