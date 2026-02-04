import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./AccentSwatch.module.css";

type AccentSwatchProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  color: string;
  label: string;
  selected?: boolean;
};

export function AccentSwatch({ color, label, selected = false, className, ...props }: AccentSwatchProps) {
  return (
    <button
      type="button"
      className={cx(styles.swatch, selected && styles.selected, className)}
      {...props}
    >
      <span className={styles.color} style={{ background: color }} />
      <Text as="span" size="sm" tone={selected ? "primary" : "secondary"}>
        {label}
      </Text>
    </button>
  );
}
