import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import { ResponseBlock } from "../ResponseBlock/ResponseBlock";
import { ResponseHeader } from "../ResponseHeader/ResponseHeader";
import styles from "./TextResponseCard.module.css";

type TextResponseCardProps = HTMLAttributes<HTMLDivElement> & {
  label?: string;
  body: string;
};

export function TextResponseCard({ label = "Assistant", body, className, ...props }: TextResponseCardProps) {
  return (
    <ResponseBlock className={cx(styles.card, className)} {...props}>
      <ResponseHeader label={label} />
      <div className={styles.body}>{body}</div>
    </ResponseBlock>
  );
}
