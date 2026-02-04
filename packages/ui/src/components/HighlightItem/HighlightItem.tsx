import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./HighlightItem.module.css";

type HighlightItemProps = HTMLAttributes<HTMLDivElement> & {
  title: string;
  description: string;
};

export function HighlightItem({ title, description, className, ...props }: HighlightItemProps) {
  return (
    <div className={cx(styles.item, className)} {...props}>
      <Text as="div" size="sm" weight="medium">
        {title}
      </Text>
      <Text as="div" size="sm" tone="secondary">
        {description}
      </Text>
    </div>
  );
}
