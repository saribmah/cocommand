import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./NavTab.module.css";

type NavTabProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  active?: boolean;
  done?: boolean;
  leading?: ReactNode;
};

export function NavTab({ label, active = false, done = false, leading, className, ...props }: NavTabProps) {
  return (
    <button
      type="button"
      className={cx(styles.tab, active && styles.active, done && styles.done, className)}
      {...props}
    >
      {leading ? <span className={styles.leading}>{leading}</span> : null}
      <Text as="span" size="xs" tone={active ? "primary" : "secondary"}>
        {label}
      </Text>
    </button>
  );
}
