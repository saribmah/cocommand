import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./IconContainer.module.css";

type IconContainerProps = HTMLAttributes<HTMLDivElement> & {
  size?: number;
};

export function IconContainer({ size = 36, className, style, ...props }: IconContainerProps) {
  return (
    <div
      className={cx(styles.container, className)}
      style={{ width: size, height: size, ...style }}
      {...props}
    />
  );
}
