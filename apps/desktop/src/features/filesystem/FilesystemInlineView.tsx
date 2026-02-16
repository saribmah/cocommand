import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import {
  Icon,
  IconContainer,
  ListItem,
  ListSection,
  Text,
} from "@cocommand/ui";
import { useFileSystemContext } from "./filesystem.context";
import type { ExtensionViewProps } from "../command/extension-views";

const FileIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
    <polyline points="14,2 14,8 20,8" />
  </svg>
);

const FolderIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" />
  </svg>
);

interface FilesystemInlineViewProps extends ExtensionViewProps {
  query?: string;
  onSelectFile?: (entry: { path: string; name: string; type: "file" | "directory" | "symlink" | "other" }) => void;
}

export function FilesystemInlineView({ query, onSelectFile }: FilesystemInlineViewProps) {
  const searchResults = useFileSystemContext((state) => state.searchResults);
  const isSearching = useFileSystemContext((state) => state.isSearching);
  const searchError = useFileSystemContext((state) => state.searchError);
  const searchFiles = useFileSystemContext((state) => state.search);
  const clearSearch = useFileSystemContext((state) => state.clearSearch);

  const [fileIndex, setFileIndex] = useState(0);
  const searchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!query?.trim()) {
      clearSearch();
      return;
    }
    if (searchDebounceRef.current) {
      clearTimeout(searchDebounceRef.current);
    }
    searchDebounceRef.current = setTimeout(() => {
      searchFiles({ query: query.trim(), maxResults: 50 });
    }, 150);
    return () => {
      if (searchDebounceRef.current) {
        clearTimeout(searchDebounceRef.current);
      }
    };
  }, [query, clearSearch, searchFiles]);

  useEffect(() => {
    setFileIndex(0);
  }, [searchResults]);

  const fileEntries = searchResults?.results ?? [];

  const handleKeyDown = (e: KeyboardEvent) => {
    if (fileEntries.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setFileIndex((idx) => (idx + 1) % fileEntries.length);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setFileIndex((idx) => (idx <= 0 ? fileEntries.length - 1 : idx - 1));
      return;
    }
    if (e.key === "Enter" && onSelectFile) {
      e.preventDefault();
      const selected = fileEntries[fileIndex];
      if (selected) {
        onSelectFile({ path: selected.path, name: selected.name, type: selected.type });
      }
    }
  };

  return (
    <div onKeyDown={handleKeyDown}>
      <ListSection label={isSearching ? "Searching..." : `Files${searchResults ? ` (${searchResults.count})` : ""}`}>
        {searchError ? (
          <Text size="sm" tone="secondary">
            {searchError}
          </Text>
        ) : fileEntries.length > 0 ? (
          fileEntries.map((entry, index) => (
            <ListItem
              key={entry.path}
              title={entry.name}
              subtitle={entry.path}
              icon={
                <IconContainer>
                  <Icon>{entry.type === "directory" ? FolderIcon : FileIcon}</Icon>
                </IconContainer>
              }
              selected={index === fileIndex}
              onMouseDown={(event) => {
                event.preventDefault();
                onSelectFile?.({ path: entry.path, name: entry.name, type: entry.type });
              }}
            />
          ))
        ) : query?.trim() ? (
          <Text size="sm" tone="secondary">
            {isSearching ? "Searching..." : "No files found."}
          </Text>
        ) : (
          <Text size="sm" tone="secondary">
            Type to search files...
          </Text>
        )}
      </ListSection>
    </div>
  );
}
