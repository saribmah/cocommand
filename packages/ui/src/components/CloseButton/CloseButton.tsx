import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { KeyHint } from "../KeyHint/KeyHint";
import { Text } from "../Text/Text";
import styles from "./CloseButton.module.css";

type CloseButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  label?: string;
  keyLabel?: string | string[];
};

export function CloseButton({
  label = "Close",
  keyLabel = "esc",
  className,
  ...props
}: CloseButtonProps) {
  return (
    <button type="button" className={cx(styles.button, className)} {...props}>
      <KeyHint keys={keyLabel} />
      <Text as="span" size="xs" tone="secondary">
        {label}
      </Text>
    </button>
  );
}
