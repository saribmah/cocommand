import type { HTMLAttributes } from "react";
import { cx } from "../../utils/classNames";
import styles from "./PanelSurface.module.css";

type PanelSurfaceProps = HTMLAttributes<HTMLDivElement> & {
  elevated?: boolean;
};

export function PanelSurface({ elevated = true, className, ...props }: PanelSurfaceProps) {
  return (
    <div
      className={cx(styles.panel, elevated && styles.elevated, className)}
      {...props}
    />
  );
}
