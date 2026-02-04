import type { HTMLAttributes, ReactNode } from "react";
import { cx } from "../../utils/classNames";
import { PanelSurface } from "../PanelSurface/PanelSurface";
import styles from "./CommandPaletteShell.module.css";

type CommandPaletteShellProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

export function CommandPaletteShell({ children, className, ...props }: CommandPaletteShellProps) {
  return (
    <PanelSurface className={cx(styles.shell, className)} {...props}>
      {children}
    </PanelSurface>
  );
}
