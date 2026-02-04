import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ButtonSecondary.module.css";

type ButtonSecondaryProps = ButtonHTMLAttributes<HTMLButtonElement>;

export function ButtonSecondary({ className, ...props }: ButtonSecondaryProps) {
  return <button type="button" className={cx(styles.button, className)} {...props} />;
}
