import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { ListSectionHeader } from "../ListSectionHeader/ListSectionHeader";
import styles from "./ListSection.module.css";

type ListSectionProps = HTMLAttributes<HTMLDivElement> & {
  label: string;
  children: ReactNode;
};

export function ListSection({ label, children, className, ...props }: ListSectionProps) {
  return (
    <section className={cx(styles.section, className)} {...props}>
      <ListSectionHeader label={label} />
      <div className={styles.items}>{children}</div>
    </section>
  );
}
