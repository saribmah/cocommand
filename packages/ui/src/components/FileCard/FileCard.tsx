import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { ActionRow } from "../ActionRow/ActionRow";
import { ResponseBlock } from "../ResponseBlock/ResponseBlock";
import { ResponseHeader } from "../ResponseHeader/ResponseHeader";
import { Text } from "../Text/Text";
import styles from "./FileCard.module.css";

type FileCardProps = HTMLAttributes<HTMLDivElement> & {
  fileName: string;
  fileType?: string;
  fileSize?: string;
  actions?: ReactNode;
};

export function FileCard({
  fileName,
  fileType,
  fileSize,
  actions,
  className,
  ...props
}: FileCardProps) {
  const metaItems = [fileType, fileSize].filter(Boolean).join(" • ");

  return (
    <ResponseBlock className={cx(styles.card, className)} {...props}>
      <ResponseHeader label="File" />
      <div className={styles.body}>
        <div>
          <Text as="div" size="sm" weight="medium">
            {fileName}
          </Text>
          {metaItems ? (
            <Text as="div" size="xs" tone="tertiary">
              {metaItems}
            </Text>
          ) : null}
        </div>
        {actions ? <ActionRow>{actions}</ActionRow> : null}
      </div>
    </ResponseBlock>
  );
}
