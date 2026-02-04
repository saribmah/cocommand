import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ListSectionHeader.module.css";

type ListSectionHeaderProps = HTMLAttributes<HTMLDivElement> & {
  label: string;
};

export function ListSectionHeader({ label, className, ...props }: ListSectionHeaderProps) {
  return (
    <div className={cx(styles.header, className)} {...props}>
      <Text as="span" size="xs" tone="tertiary">
        {label}
      </Text>
    </div>
  );
}
