import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./Field.module.css";

type FieldProps = HTMLAttributes<HTMLDivElement> & {
  label?: string;
  help?: string;
  error?: string;
  children: ReactNode;
};

export function Field({ label, help, error, children, className, ...props }: FieldProps) {
  return (
    <div className={cx(styles.field, className)} {...props}>
      {label ? (
        <Text as="label" size="xs" tone="tertiary" className={styles.label}>
          {label}
        </Text>
      ) : null}
      <div className={styles.control}>{children}</div>
      {help ? (
        <Text as="div" size="xs" tone="secondary" className={styles.help}>
          {help}
        </Text>
      ) : null}
      {error ? (
        <Text as="div" size="xs" tone="secondary" className={styles.error}>
          {error}
        </Text>
      ) : null}
    </div>
  );
}
