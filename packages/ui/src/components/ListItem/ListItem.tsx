import { useEffect, useRef, type HTMLAttributes, type ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ListItem.module.css";

type ListItemProps = HTMLAttributes<HTMLDivElement> & {
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  rightMeta?: ReactNode;
  selected?: boolean;
};

export function ListItem({
  title,
  subtitle,
  icon,
  rightMeta,
  selected = false,
  className,
  ...props
}: ListItemProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (selected && ref.current) {
      ref.current.scrollIntoView({ block: "nearest" });
    }
  }, [selected]);

  return (
    <div
      ref={ref}
      className={cx(styles.item, selected && styles.selected, className)}
      {...props}
    >
      {icon ? <div className={styles.icon}>{icon}</div> : null}
      <div className={styles.content}>
        <Text as="div" size="md" weight="medium" className={styles.title}>
          {title}
        </Text>
        {subtitle ? (
          <Text as="div" size="sm" tone="secondary" className={styles.subtitle}>
            {subtitle}
          </Text>
        ) : null}
      </div>
      {rightMeta ? <div className={styles.meta}>{rightMeta}</div> : null}
    </div>
  );
}
