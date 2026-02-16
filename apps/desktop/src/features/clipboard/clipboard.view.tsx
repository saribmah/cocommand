import { ClipboardSidebar } from "./components/ClipboardSidebar";
import { ClipboardDetail } from "./components/ClipboardDetail";
import styles from "./clipboard.module.css";
import type { ExtensionViewProps } from "../extension/extension-views";

export function ClipboardView({ mode }: ExtensionViewProps) {
  const isInline = mode === "inline";
  return (
    <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
      <ClipboardSidebar />
      <ClipboardDetail />
    </div>
  );
}
