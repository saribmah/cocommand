import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { Text } from "../Text/Text";
import styles from "./ErrorBanner.module.css";

type ErrorBannerProps = HTMLAttributes<HTMLDivElement> & {
  message: string;
};

export function ErrorBanner({ message, className, ...props }: ErrorBannerProps) {
  return (
    <div className={cx(styles.banner, className)} role="alert" {...props}>
      <Text as="span" size="sm" tone="secondary">
        {message}
      </Text>
    </div>
  );
}
