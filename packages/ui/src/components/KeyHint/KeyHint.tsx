import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Pill } from "../Pill/Pill";
import styles from "./KeyHint.module.css";

type KeyHintProps = HTMLAttributes<HTMLSpanElement> & {
  keys: string | string[];
};

export function KeyHint({ keys, className, ...props }: KeyHintProps) {
  const items = Array.isArray(keys) ? keys : [keys];

  return (
    <Pill className={cx(styles.hint, className)} {...props}>
      {items.map((key, index) => (
        <span className={styles.key} key={`${key}-${index}`}>
          {key}
        </span>
      ))}
    </Pill>
  );
}
