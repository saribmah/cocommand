import { Text } from "@cocommand/ui";
import type { SearchEntry } from "../filesystem.types";
import styles from "./FileDetail.module.css";

const FileIconLarge = (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
    <polyline points="14,2 14,8 20,8" />
  </svg>
);

const FolderIconLarge = (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
  </svg>
);

function formatFileSize(bytes: number | null): string {
  if (bytes === null || bytes < 0) return "—";
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const k = 1024;
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const value = bytes / k ** i;
  return `${value < 10 && i > 0 ? value.toFixed(1) : Math.round(value)} ${units[i]}`;
}

function formatModifiedDate(timestamp: number | null): string {
  if (!timestamp) return "—";
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString("en-US", {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}

function getExtension(name: string): string | null {
  const dotIndex = name.lastIndexOf(".");
  if (dotIndex <= 0) return null;
  return name.slice(dotIndex);
}

function formatEntryType(type: SearchEntry["type"]): string {
  switch (type) {
    case "file":
      return "File";
    case "directory":
      return "Directory";
    case "symlink":
      return "Symlink";
    default:
      return "Other";
  }
}

interface FileDetailProps {
  entry: SearchEntry | null;
}

export function FileDetail({ entry }: FileDetailProps) {
  if (!entry) {
    return (
      <div className={styles.panel}>
        <div className={styles.empty}>
          <Text as="div" size="sm" tone="secondary">
            Select a file to view details
          </Text>
        </div>
      </div>
    );
  }

  const ext = getExtension(entry.name);

  return (
    <div className={styles.panel}>
      <div className={styles.content}>
        <div className={styles.header}>
          <span className={styles.headerIcon}>
            {entry.type === "directory" ? FolderIconLarge : FileIconLarge}
          </span>
          <span className={styles.fileName}>{entry.name}</span>
        </div>

        <div className={styles.metaSection}>
          <div className={styles.metaRow}>
            <span className={styles.metaLabel}>Path</span>
            <span className={styles.metaValue} title={entry.path}>{entry.path}</span>
          </div>
          <div className={styles.metaRow}>
            <span className={styles.metaLabel}>Type</span>
            <span className={styles.metaValue}>{formatEntryType(entry.type)}</span>
          </div>
          {ext && (
            <div className={styles.metaRow}>
              <span className={styles.metaLabel}>Extension</span>
              <span className={styles.metaValue}>{ext}</span>
            </div>
          )}
          <div className={styles.metaRow}>
            <span className={styles.metaLabel}>Size</span>
            <span className={styles.metaValue}>{formatFileSize(entry.size)}</span>
          </div>
          <div className={styles.metaRow}>
            <span className={styles.metaLabel}>Modified</span>
            <span className={styles.metaValue}>{formatModifiedDate(entry.modifiedAt)}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
