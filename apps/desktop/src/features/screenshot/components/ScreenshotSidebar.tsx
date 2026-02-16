import { useEffect } from "react";
import { Text } from "@cocommand/ui";
import { useStore } from "zustand";
import { useExtensionStore } from "../../extension/extension.context";
import { useServerContext } from "../../server/server.context";
import type { ScreenshotState } from "../screenshot.store";
import styles from "./ScreenshotSidebar.module.css";

function formatRelativeTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (
    date.getDate() === now.getDate() &&
    date.getMonth() === now.getMonth() &&
    date.getFullYear() === now.getFullYear()
  ) {
    return date.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    });
  }

  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (
    date.getDate() === yesterday.getDate() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getFullYear() === yesterday.getFullYear()
  ) {
    return "Yesterday";
  }

  if (days < 7) {
    return date.toLocaleDateString("en-US", { weekday: "long" });
  }

  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
  });
}

export function ScreenshotSidebar() {
  const screenshotStore = useExtensionStore<ScreenshotState>("screenshot");
  const screenshots = useStore(screenshotStore, (s) => s.screenshots);
  const selectedFilename = useStore(screenshotStore, (s) => s.selectedFilename);
  const isLoading = useStore(screenshotStore, (s) => s.isLoading);
  const fetchScreenshots = useStore(screenshotStore, (s) => s.fetchScreenshots);
  const selectScreenshot = useStore(screenshotStore, (s) => s.selectScreenshot);
  const serverInfo = useServerContext((state) => state.info);

  useEffect(() => {
    fetchScreenshots();
  }, [fetchScreenshots]);

  const serverAddr = serverInfo?.addr;
  const buildImageUrl = (filename: string) => {
    const prefix = serverAddr?.startsWith("http")
      ? serverAddr
      : `http://${serverAddr}`;
    return `${prefix}/workspace/screenshots/${encodeURIComponent(filename)}`;
  };

  return (
    <div className={styles.sidebar}>
      <div className={styles.header}>
        <Text as="div" size="md" weight="semibold">
          Screenshots{" "}
          <span className={styles.entryCount}>{screenshots.length}</span>
        </Text>
      </div>
      <div className={styles.list}>
        {isLoading && screenshots.length === 0 ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              Loading...
            </Text>
          </div>
        ) : screenshots.length === 0 ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              No screenshots
            </Text>
          </div>
        ) : (
          screenshots.map((entry) => (
            <button
              key={entry.filename}
              className={`${styles.entryItem} ${
                entry.filename === selectedFilename
                  ? styles.entryItemSelected
                  : ""
              }`}
              onClick={() => selectScreenshot(entry.filename)}
              type="button"
            >
              {serverAddr && (
                <img
                  className={styles.thumbnail}
                  src={buildImageUrl(entry.filename)}
                  alt={entry.filename}
                  loading="lazy"
                />
              )}
              <div className={styles.entryContent}>
                <span className={styles.entryFilename}>
                  {entry.filename}
                </span>
                <div className={styles.entryMeta}>
                  <span className={styles.entryDate}>
                    {formatRelativeTime(entry.created_at)}
                  </span>
                  <span className={styles.formatBadge}>{entry.format}</span>
                </div>
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
