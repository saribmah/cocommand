import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ChoiceCard.module.css";

type ChoiceCardProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  title: string;
  description?: string;
  selected?: boolean;
};

export function ChoiceCard({
  title,
  description,
  selected = false,
  className,
  ...props
}: ChoiceCardProps) {
  return (
    <button
      type="button"
      className={cx(styles.card, selected && styles.selected, className)}
      {...props}
    >
      <Text as="span" size="xs" tone={selected ? "primary" : "secondary"}>
        {title}
      </Text>
      {description ? (
        <Text as="span" size="xs" tone="secondary" className={styles.description}>
          {description}
        </Text>
      ) : null}
    </button>
  );
}
