import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./ButtonPrimary.module.css";

type ButtonPrimaryProps = ButtonHTMLAttributes<HTMLButtonElement>;

export function ButtonPrimary({ className, ...props }: ButtonPrimaryProps) {
  return <button type="button" className={cx(styles.button, className)} {...props} />;
}
