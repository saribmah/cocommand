import type { InputHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./TextInput.module.css";

type TextInputProps = InputHTMLAttributes<HTMLInputElement>;

export function TextInput({ className, ...props }: TextInputProps) {
  return <input className={cx(styles.input, className)} {...props} />;
}
