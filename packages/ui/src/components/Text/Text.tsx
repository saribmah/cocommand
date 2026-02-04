import type { ElementType, HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./Text.module.css";

type TextTone = "primary" | "secondary" | "tertiary";
type TextSize = "xs" | "sm" | "md" | "lg";
type TextWeight = "regular" | "medium" | "semibold";

type TextProps<T extends ElementType> = {
  as?: T;
  tone?: TextTone;
  size?: TextSize;
  weight?: TextWeight;
} & HTMLAttributes<HTMLElement>;

export function Text<T extends ElementType = "span">({
  as,
  tone = "primary",
  size = "md",
  weight = "regular",
  className,
  ...props
}: TextProps<T>) {
  const Component = (as ?? "span") as ElementType;

  return (
    <Component
      className={cx(
        styles.text,
        styles[`tone-${tone}`],
        styles[`size-${size}`],
        styles[`weight-${weight}`],
        className
      )}
      {...props}
    />
  );
}
