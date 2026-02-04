import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { ResponseBlock } from "../ResponseBlock/ResponseBlock";
import { ResponseHeader } from "../ResponseHeader/ResponseHeader";
import { Text } from "../Text/Text";
import styles from "./ErrorCard.module.css";

type ErrorCardProps = HTMLAttributes<HTMLDivElement> & {
  message: string;
  label?: string;
};

export function ErrorCard({ message, label = "Error", className, ...props }: ErrorCardProps) {
  return (
    <ResponseBlock className={cx(styles.card, className)} {...props}>
      <ResponseHeader label={label} />
      <Text as="div" size="sm" tone="secondary" className={styles.message}>
        {message}
      </Text>
    </ResponseBlock>
  );
}
