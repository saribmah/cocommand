import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./HintItem.module.css";

type HintItemProps = HTMLAttributes<HTMLDivElement> & {
  label: string;
  keyHint?: ReactNode;
};

export function HintItem({ label, keyHint, className, ...props }: HintItemProps) {
  return (
    <div className={cx(styles.item, className)} {...props}>
      {keyHint ? <div className={styles.key}>{keyHint}</div> : null}
      <Text as="span" size="xs" tone="secondary">
        {label}
      </Text>
    </div>
  );
}
