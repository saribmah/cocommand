import { ScreenshotSidebar } from "./components/ScreenshotSidebar";
import { ScreenshotDetail } from "./components/ScreenshotDetail";
import styles from "./screenshot.module.css";
import type { ExtensionViewProps } from "../extension/extension-views";

export function ScreenshotView({ mode }: ExtensionViewProps) {
  const isInline = mode === "inline";
  return (
    <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
      <ScreenshotSidebar />
      <ScreenshotDetail />
    </div>
  );
}
