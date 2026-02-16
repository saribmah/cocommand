import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Text } from "@cocommand/ui";
import { useFileSystemContext } from "../filesystem.context";
import type { SearchEntry } from "../filesystem.types";
import styles from "./file-list.module.css";

const FileIcon = (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
    <polyline points="14,2 14,8 20,8" />
  </svg>
);

const FolderIcon = (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
  </svg>
);

interface FileListProps {
  onSelect: (entry: SearchEntry) => void;
  onActivate: (entry: SearchEntry) => void;
}

export function FileList({ onSelect, onActivate }: FileListProps) {
  const searchResults = useFileSystemContext((state) => state.searchResults);
  const isSearching = useFileSystemContext((state) => state.isSearching);
  const searchError = useFileSystemContext((state) => state.searchError);
  const searchFiles = useFileSystemContext((state) => state.search);
  const clearSearch = useFileSystemContext((state) => state.clearSearch);

  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      clearSearch();
      return;
    }
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      searchFiles({ query: trimmed, maxResults: 50 });
    }, 150);
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [query, clearSearch, searchFiles]);

  const entries = searchResults?.results ?? [];

  useEffect(() => {
    setSelectedIndex(0);
  }, [searchResults]);

  useEffect(() => {
    if (entries.length > 0 && selectedIndex < entries.length) {
      const entry = entries[selectedIndex];
      if (entry) onSelect(entry);
    }
  }, [selectedIndex, entries]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (entries.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((idx) => (idx + 1) % entries.length);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((idx) => (idx <= 0 ? entries.length - 1 : idx - 1));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const selected = entries[selectedIndex];
      if (selected) {
        onActivate(selected);
      }
    }
  };

  return (
    <div className={styles.sidebar}>
      <div className={styles.searchContainer}>
        <input
          ref={inputRef}
          type="text"
          className={styles.searchInput}
          placeholder="Search files..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          spellCheck={false}
          autoComplete="off"
        />
      </div>
      <div className={styles.fileList}>
        {searchError ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              {searchError}
            </Text>
          </div>
        ) : entries.length > 0 ? (
          entries.map((entry, index) => (
            <button
              key={entry.path}
              type="button"
              className={`${styles.fileItem} ${index === selectedIndex ? styles.fileItemSelected : ""}`}
              onClick={() => {
                setSelectedIndex(index);
                onSelect(entry);
              }}
              onDoubleClick={() => onActivate(entry)}
            >
              <span className={styles.fileItemIcon}>
                {entry.type === "directory" ? FolderIcon : FileIcon}
              </span>
              <div className={styles.fileItemInfo}>
                <span className={styles.fileItemName}>{entry.name}</span>
                <span className={styles.fileItemPath}>{entry.path}</span>
              </div>
            </button>
          ))
        ) : query.trim() ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              {isSearching ? "Searching..." : "No files found."}
            </Text>
          </div>
        ) : (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              Type to search files...
            </Text>
          </div>
        )}
      </div>
    </div>
  );
}
