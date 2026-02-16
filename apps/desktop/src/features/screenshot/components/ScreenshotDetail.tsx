import { Text } from "@cocommand/ui";
import { useStore } from "zustand";
import { useExtensionStore } from "../../extension/extension.context";
import { useServerContext } from "../../server/server.context";
import type { ScreenshotState } from "../screenshot.store";
import styles from "./ScreenshotDetail.module.css";

function formatFullDate(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return (
    date.toLocaleDateString("en-US", {
      weekday: "short",
      month: "short",
      day: "numeric",
      year: "numeric",
    }) +
    " at " +
    date.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    })
  );
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ScreenshotDetail() {
  const screenshotStore = useExtensionStore<ScreenshotState>("screenshot");
  const screenshots = useStore(screenshotStore, (s) => s.screenshots);
  const selectedFilename = useStore(screenshotStore, (s) => s.selectedFilename);
  const copyToClipboard = useStore(screenshotStore, (s) => s.copyToClipboard);
  const deleteScreenshot = useStore(screenshotStore, (s) => s.deleteScreenshot);
  const serverInfo = useServerContext((state) => state.info);

  const entry = screenshots.find((s) => s.filename === selectedFilename) ?? null;

  if (!entry) {
    return (
      <div className={styles.empty}>
        <Text as="div" size="md" tone="secondary">
          Select a screenshot to view
        </Text>
      </div>
    );
  }

  const serverAddr = serverInfo?.addr;
  const imageUrl = serverAddr
    ? `${serverAddr.startsWith("http") ? serverAddr : `http://${serverAddr}`}/workspace/screenshots/${encodeURIComponent(entry.filename)}`
    : null;

  const handleCopy = async () => {
    try {
      await copyToClipboard(entry);
    } catch (error) {
      console.error("Failed to copy screenshot to clipboard:", error);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteScreenshot(entry.filename);
    } catch (error) {
      console.error("Failed to delete screenshot:", error);
    }
  };

  return (
    <div className={styles.detail}>
      <div className={styles.detailHeader}>
        <div className={styles.detailHeaderLeft}>
          <span className={styles.filename}>{entry.filename}</span>
          <span className={styles.formatBadge}>{entry.format}</span>
          <span className={styles.detailDate}>
            {formatFullDate(entry.created_at)}
          </span>
          <span className={styles.fileSize}>
            {formatFileSize(entry.size)}
          </span>
        </div>
        <div className={styles.headerActions}>
          <button
            className={styles.actionButton}
            onClick={handleCopy}
            type="button"
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
              <rect x="5" y="5" width="9" height="9" rx="1.5" stroke="currentColor" strokeWidth="1.3" />
              <path d="M11 5V3.5C11 2.67 10.33 2 9.5 2H3.5C2.67 2 2 2.67 2 3.5V9.5C2 10.33 2.67 11 3.5 11H5" stroke="currentColor" strokeWidth="1.3" />
            </svg>
            Copy
          </button>
          <button
            className={styles.deleteButton}
            onClick={handleDelete}
            type="button"
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
              <path d="M3 4H13M6 4V3H10V4M5 4V13H11V4" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
              <path d="M7 7V10.5M9 7V10.5" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
            </svg>
            Delete
          </button>
        </div>
      </div>
      <div className={styles.content}>
        {imageUrl ? (
          <img
            className={styles.screenshotImage}
            src={imageUrl}
            alt={entry.filename}
          />
        ) : (
          <Text as="div" size="sm" tone="secondary">
            Unable to load image
          </Text>
        )}
      </div>
    </div>
  );
}
