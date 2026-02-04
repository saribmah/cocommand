import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./AppHeader.module.css";

type AppHeaderProps = HTMLAttributes<HTMLDivElement> & {
  title: string;
  subtitle?: string;
  brand?: ReactNode;
  meta?: ReactNode;
  kicker?: string | null;
};

export function AppHeader({
  title,
  subtitle,
  brand,
  meta,
  kicker = "Cocommand",
  className,
  ...props
}: AppHeaderProps) {
  return (
    <header className={cx(styles.header, className)} {...props}>
      <div className={styles.brand}>
        {brand ? <div className={styles.mark}>{brand}</div> : null}
        <div>
          {kicker ? (
            <Text as="div" size="xs" tone="tertiary" className={styles.kicker}>
              {kicker}
            </Text>
          ) : null}
          <Text as="div" size="lg" weight="semibold">
            {title}
          </Text>
          {subtitle ? (
            <Text as="div" size="sm" tone="secondary" className={styles.subtitle}>
              {subtitle}
            </Text>
          ) : null}
        </div>
      </div>
      {meta ? <div className={styles.meta}>{meta}</div> : null}
    </header>
  );
}
