import type { HTMLAttributes, InputHTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { KeyHint } from "../KeyHint/KeyHint";
import styles from "./SearchField.module.css";

type SearchFieldProps = HTMLAttributes<HTMLDivElement> & {
  icon?: ReactNode;
  placeholder?: string;
  shortcut?: string | string[];
  inputProps?: Omit<InputHTMLAttributes<HTMLInputElement>, "className" | "placeholder">;
};

export function SearchField({
  icon,
  placeholder = "Type a command or search",
  shortcut,
  inputProps,
  className,
  ...props
}: SearchFieldProps) {
  return (
    <div className={cx(styles.field, className)} {...props}>
      {icon ? <span className={styles.icon}>{icon}</span> : null}
      <input
        type="text"
        className={styles.input}
        placeholder={placeholder}
        {...inputProps}
      />
      {shortcut ? (
        <KeyHint keys={shortcut} className={styles.shortcut} />
      ) : null}
    </div>
  );
}
