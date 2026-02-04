import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./InlineHelp.module.css";

type InlineHelpProps = HTMLAttributes<HTMLDivElement> & {
  text: string;
};

export function InlineHelp({ text, className, ...props }: InlineHelpProps) {
  return (
    <Text as="div" size="xs" tone="secondary" className={cx(styles.help, className)} {...props}>
      {text}
    </Text>
  );
}
