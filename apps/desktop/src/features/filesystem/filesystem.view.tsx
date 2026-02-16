import { useState } from "react";
import type { ExtensionViewProps } from "../command/extension-views";
import type { SearchEntry } from "./filesystem.types";
import { FileList } from "./components/file-list";
import { FileDetail } from "./components/file-detail";
import styles from "./filesystem.module.css";

interface FileSystemViewProps extends ExtensionViewProps {
  onSelectFile?: (entry: { path: string; name: string; type: "file" | "directory" | "symlink" | "other" }) => void;
}

export function FileSystemView({ mode, onSelectFile }: FileSystemViewProps) {
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
