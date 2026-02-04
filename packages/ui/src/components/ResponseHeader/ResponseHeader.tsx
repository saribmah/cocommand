import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ResponseHeader.module.css";

type ResponseHeaderProps = HTMLAttributes<HTMLDivElement> & {
  icon?: ReactNode;
  label: string;
  meta?: ReactNode;
  actions?: ReactNode;
};

export function ResponseHeader({ icon, label, meta, actions, className, ...props }: ResponseHeaderProps) {
  return (
    <div className={cx(styles.header, className)} {...props}>
      <div className={styles.left}>
        {icon ? <span className={styles.icon}>{icon}</span> : null}
        <Text as="span" size="sm" weight="medium">
          {label}
        </Text>
        {meta ? <div className={styles.meta}>{meta}</div> : null}
      </div>
      {actions ? <div className={styles.actions}>{actions}</div> : null}
    </div>
  );
}
