import type { HTMLAttributes, InputHTMLAttributes, ReactNode, Ref } from "react";
import { cx } from "../../utils/classNames";
import { KeyHint } from "../KeyHint/KeyHint";
import styles from "./SearchField.module.css";

type SearchFieldProps = HTMLAttributes<HTMLDivElement> & {
  icon?: ReactNode;
  placeholder?: string;
  shortcut?: string | string[];
  inputRef?: Ref<HTMLInputElement>;
  inputProps?: Omit<InputHTMLAttributes<HTMLInputElement>, "className" | "placeholder">;
};

export function SearchField({
  icon,
  placeholder = "Type a command or search",
  shortcut,
  inputRef,
  inputProps,
  className,
  ...props
}: SearchFieldProps) {
  return (
    <div className={cx(styles.field, className)} {...props}>
      {icon ? <span className={styles.icon}>{icon}</span> : null}
      <input
        ref={inputRef}
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
