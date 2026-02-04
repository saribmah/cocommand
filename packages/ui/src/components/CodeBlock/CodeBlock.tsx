import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./CodeBlock.module.css";

type CodeBlockProps = HTMLAttributes<HTMLPreElement> & {
  code: string;
};

export function CodeBlock({ code, className, ...props }: CodeBlockProps) {
  return (
    <pre className={cx(styles.block, className)} {...props}>
      <code>{code}</code>
    </pre>
  );
}
