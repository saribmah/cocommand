import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { PanelSurface } from "../PanelSurface/PanelSurface";
import styles from "./AppPanel.module.css";

type AppPanelProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function AppPanel({ children, className, ...props }: AppPanelProps) {
  return (
    <PanelSurface className={cx(styles.panel, className)} {...props}>
      {children}
    </PanelSurface>
  );
}
