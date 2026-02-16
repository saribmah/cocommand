import { useEffect } from "react";
import { Text } from "@cocommand/ui";
import { useStore } from "zustand";
import { useExtensionStore } from "../../extension/extension.context";
import type { ClipboardState } from "../clipboard.store";
import type { ClipboardEntry } from "../clipboard.types";
import styles from "./ClipboardSidebar.module.css";

function formatRelativeTime(isoString: string): string {
  const date = new Date(isoString);
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

function entryPreview(entry: ClipboardEntry): string {
  if (entry.kind === "text" && entry.text) {
    return entry.text.slice(0, 60).replace(/\n/g, " ");
  }
  if (entry.kind === "image") {
    return "Image";
  }
  if (entry.kind === "files" && entry.files) {
    return `${entry.files.length} file${entry.files.length !== 1 ? "s" : ""}`;
  }
  return "Empty";
}

function KindIcon({ kind }: { kind: ClipboardEntry["kind"] }) {
  if (kind === "text") {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
        <path d="M4 3H12V5H9V13H7V5H4V3Z" fill="currentColor" />
      </svg>
    );
  }
  if (kind === "image") {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
        <rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor" strokeWidth="1.3" />
        <circle cx="5.5" cy="6.5" r="1.5" fill="currentColor" />
        <path d="M2 11L5.5 8L8 10.5L10.5 8.5L14 11" stroke="currentColor" strokeWidth="1.3" strokeLinejoin="round" />
      </svg>
    );
  }
  // files
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
      <path d="M2 3.5C2 2.67 2.67 2 3.5 2H6.29C6.69 2 7.07 2.16 7.35 2.44L8.56 3.65C8.84 3.93 9.22 4.09 9.62 4.09H12.5C13.33 4.09 14 4.76 14 5.59V12.5C14 13.33 13.33 14 12.5 14H3.5C2.67 14 2 13.33 2 12.5V3.5Z" stroke="currentColor" strokeWidth="1.3" />
    </svg>
  );
}

export function ClipboardSidebar() {
  const clipboardStore = useExtensionStore<ClipboardState>("clipboard");
  const entries = useStore(clipboardStore, (s) => s.entries);
  const selectedEntryId = useStore(clipboardStore, (s) => s.selectedEntryId);
  const isLoading = useStore(clipboardStore, (s) => s.isLoading);
  const fetchHistory = useStore(clipboardStore, (s) => s.fetchHistory);
  const selectEntry = useStore(clipboardStore, (s) => s.selectEntry);
  const clearHistory = useStore(clipboardStore, (s) => s.clearHistory);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const handleClearHistory = async () => {
    try {
      await clearHistory();
    } catch (error) {
      console.error("Failed to clear clipboard history:", error);
    }
  };

  return (
    <div className={styles.sidebar}>
      <div className={styles.header}>
        <Text as="div" size="md" weight="semibold">
          Clipboard{" "}
          <span className={styles.entryCount}>{entries.length}</span>
        </Text>
        <div className={styles.headerActions}>
          <button
            className={styles.iconButton}
            onClick={handleClearHistory}
            aria-label="Clear history"
            type="button"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path d="M3 4H13M6 4V3H10V4M5 4V13H11V4" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
              <path d="M7 7V10.5M9 7V10.5" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      </div>
      <div className={styles.list}>
        {isLoading && entries.length === 0 ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              Loading...
            </Text>
          </div>
        ) : entries.length === 0 ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              No clipboard history
            </Text>
          </div>
        ) : (
          entries.map((entry) => (
            <button
              key={entry.id}
              className={`${styles.entryItem} ${
                entry.id === selectedEntryId ? styles.entryItemSelected : ""
              }`}
              onClick={() => selectEntry(entry.id)}
              type="button"
            >
              <span className={styles.entryIcon}>
                <KindIcon kind={entry.kind} />
              </span>
              <div className={styles.entryContent}>
                <span className={styles.entryPreview}>
                  {entryPreview(entry)}
                </span>
                <span className={styles.entryDate}>
                  {formatRelativeTime(entry.created_at)}
                </span>
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
