import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ActionHint.module.css";

type ActionHintProps = HTMLAttributes<HTMLSpanElement> & {
  label: string;
  icon?: ReactNode;
};

export function ActionHint({ label, icon, className, ...props }: ActionHintProps) {
  return (
    <span className={cx(styles.hint, className)} {...props}>
      <Text as="span" size="xs" tone="secondary">
        {label}
      </Text>
      {icon ? <span className={styles.icon}>{icon}</span> : null}
    </span>
  );
}
