import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { KeyHint } from "../KeyHint/KeyHint";
import { Text } from "../Text/Text";
import styles from "./SearchField.module.css";

type SearchFieldProps = HTMLAttributes<HTMLDivElement> & {
  icon?: ReactNode;
  placeholder?: string;
  shortcut?: string | string[];
};

export function SearchField({
  icon,
  placeholder = "Type a command or search",
  shortcut,
  className,
  ...props
}: SearchFieldProps) {
  return (
    <div className={cx(styles.field, className)} {...props}>
      {icon ? <span className={styles.icon}>{icon}</span> : null}
      <Text as="span" tone="secondary" size="md" className={styles.placeholder}>
        {placeholder}
      </Text>
      {shortcut ? (
        <KeyHint keys={shortcut} className={styles.shortcut} />
      ) : null}
    </div>
  );
}
