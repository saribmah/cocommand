import { Text } from "@cocommand/ui";
import { useStore } from "zustand";
import { useExtensionStore } from "../../extension/extension.context";
import type { ClipboardState } from "../clipboard.store";
import styles from "./ClipboardDetail.module.css";

function formatFullDate(isoString: string): string {
  const date = new Date(isoString);
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

export function ClipboardDetail() {
  const clipboardStore = useExtensionStore<ClipboardState>("clipboard");
  const entries = useStore(clipboardStore, (s) => s.entries);
  const selectedEntryId = useStore(clipboardStore, (s) => s.selectedEntryId);
  const copyToClipboard = useStore(clipboardStore, (s) => s.copyToClipboard);

  const entry = entries.find((e) => e.id === selectedEntryId) ?? null;

  if (!entry) {
    return (
      <div className={styles.empty}>
        <Text as="div" size="md" tone="secondary">
          Select an entry to view
        </Text>
      </div>
    );
  }

  const handleCopy = async () => {
    try {
      await copyToClipboard(entry);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  return (
    <div className={styles.detail}>
      <div className={styles.detailHeader}>
        <div className={styles.detailHeaderLeft}>
          <span className={styles.kindBadge}>{entry.kind}</span>
          <span className={styles.detailDate}>
            {formatFullDate(entry.created_at)}
          </span>
        </div>
        <button
          className={styles.copyButton}
          onClick={handleCopy}
          type="button"
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none">
            <rect x="5" y="5" width="9" height="9" rx="1.5" stroke="currentColor" strokeWidth="1.3" />
            <path d="M11 5V3.5C11 2.67 10.33 2 9.5 2H3.5C2.67 2 2 2.67 2 3.5V9.5C2 10.33 2.67 11 3.5 11H5" stroke="currentColor" strokeWidth="1.3" />
          </svg>
          Copy
        </button>
      </div>
      <div className={styles.content}>
        {entry.kind === "text" && (
          <pre className={styles.textContent}>{entry.text}</pre>
        )}
        {entry.kind === "image" && (
          <div className={styles.imageInfo}>
            <span className={styles.imageLabel}>
              Image ({entry.image_format ?? "unknown format"})
            </span>
            {entry.image_path && (
              <span className={styles.imagePath}>{entry.image_path}</span>
            )}
          </div>
        )}
        {entry.kind === "files" && entry.files && (
          <ul className={styles.fileList}>
            {entry.files.map((file) => (
              <li key={file} className={styles.fileItem}>
                {file}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
