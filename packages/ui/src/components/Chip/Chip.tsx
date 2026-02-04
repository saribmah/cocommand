import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./Chip.module.css";

type ChipProps = HTMLAttributes<HTMLButtonElement> & {
  icon?: ReactNode;
  label: string;
  active?: boolean;
};

export function Chip({ icon, label, active = false, className, ...props }: ChipProps) {
  return (
    <button
      type="button"
      className={cx(styles.chip, active && styles.active, className)}
      {...props}
    >
      {icon ? <span className={styles.icon}>{icon}</span> : null}
      <Text as="span" size="xs" tone={active ? "primary" : "secondary"}>
        {label}
      </Text>
    </button>
  );
}
