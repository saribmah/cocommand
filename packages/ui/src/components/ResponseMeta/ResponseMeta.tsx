import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ResponseMeta.module.css";

type ResponseMetaProps = HTMLAttributes<HTMLDivElement> & {
  items: string[];
};

export function ResponseMeta({ items, className, ...props }: ResponseMetaProps) {
  return (
    <div className={cx(styles.meta, className)} {...props}>
      {items.map((item, index) => (
        <Text as="span" size="xs" tone="tertiary" key={`${item}-${index}`}>
          {item}
        </Text>
      ))}
    </div>
  );
}
