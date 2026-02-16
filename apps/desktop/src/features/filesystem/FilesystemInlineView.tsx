import { useState } from "react";
import type { ExtensionViewProps } from "../command/extension-views";
import type { SearchEntry } from "./filesystem.types";
import { FileList } from "./components/FileList";
import { FileDetail } from "./components/FileDetail";
import styles from "./filesystem.module.css";

interface FilesystemInlineViewProps extends ExtensionViewProps {
  onSelectFile?: (entry: { path: string; name: string; type: "file" | "directory" | "symlink" | "other" }) => void;
}

export function FilesystemInlineView({ mode, onSelectFile }: FilesystemInlineViewProps) {
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
