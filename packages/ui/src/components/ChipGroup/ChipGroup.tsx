import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ChipGroup.module.css";

type ChipGroupProps = HTMLAttributes<HTMLDivElement>;

export function ChipGroup({ className, ...props }: ChipGroupProps) {
  return <div className={cx(styles.group, className)} {...props} />;
}
