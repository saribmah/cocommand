import { useState } from "react";
import type { ExtensionViewProps } from "../extension/extension-views";
import type { SearchEntry } from "./filesystem.types";
import { FileList } from "./components/file-list";
import { FileDetail } from "./components/file-detail";
import styles from "./filesystem.module.css";

export function FileSystemView({ mode, onSelectFile }: ExtensionViewProps) {
  const isInline = mode === "inline";
  const [selectedEntry, setSelectedEntry] = useState<SearchEntry | null>(null);

  return (
    <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
      <FileList
        onSelect={setSelectedEntry}
        onActivate={(entry) => onSelectFile?.({ path: entry.path, name: entry.name, type: entry.type })}
      />
      <FileDetail entry={selectedEntry} />
    </div>
  );
}
