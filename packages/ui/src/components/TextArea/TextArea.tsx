import type { TextareaHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./TextArea.module.css";

type TextAreaProps = TextareaHTMLAttributes<HTMLTextAreaElement>;

export function TextArea({ className, ...props }: TextAreaProps) {
  return <textarea className={cx(styles.textarea, className)} {...props} />;
}
