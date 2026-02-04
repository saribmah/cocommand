import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import styles from "./Icon.module.css";

type IconProps = HTMLAttributes<HTMLSpanElement> & {
  size?: number;
  children: ReactNode;
};

export function Icon({ size = 18, className, style, children, ...props }: IconProps) {
  return (
    <span
      className={cx(styles.icon, className)}
      style={{ width: size, height: size, ...style }}
      aria-hidden
      {...props}
    >
      {children}
    </span>
  );
}
