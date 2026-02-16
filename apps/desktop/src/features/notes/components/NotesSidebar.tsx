import { useEffect } from "react";
import { Text } from "@cocommand/ui";
import { useStore } from "zustand";
import { useExtensionStore } from "../../extension/extension.context";
import type { NotesState } from "../notes.store";
import type { NoteSummary } from "../notes.types";
import styles from "./NotesSidebar.module.css";

function formatRelativeTime(timestamp: number | null): string {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  // Today: show time like "1:30 PM"
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

  // Yesterday
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (
    date.getDate() === yesterday.getDate() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getFullYear() === yesterday.getFullYear()
  ) {
    return "Yesterday";
  }

  // Within this week
  if (days < 7) {
    return date.toLocaleDateString("en-US", { weekday: "long" });
  }

  // Older: show date
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
  });
}

export function NotesSidebar() {
  const notesStore = useExtensionStore<NotesState>("notes");
  const notes = useStore(notesStore, (s) => s.notes);
  const selectedNoteId = useStore(notesStore, (s) => s.selectedNoteId);
  const isLoading = useStore(notesStore, (s) => s.isLoading);
  const fetchNotes = useStore(notesStore, (s) => s.fetchNotes);
  const selectNote = useStore(notesStore, (s) => s.selectNote);
  const createNote = useStore(notesStore, (s) => s.createNote);

  useEffect(() => {
    fetchNotes();
  }, [fetchNotes]);

  const handleCreateNote = async () => {
    try {
      await createNote();
    } catch (error) {
      console.error("Failed to create note:", error);
    }
  };

  const handleSelectNote = (note: NoteSummary) => {
    selectNote(note.id);
  };

  return (
    <div className={styles.sidebar}>
      <div className={styles.header}>
        <Text as="div" size="md" weight="semibold">
          Notes{" "}
          <span className={styles.noteCount}>{notes.length}</span>
        </Text>
        <div className={styles.headerActions}>
          <button
            className={styles.iconButton}
            aria-label="Search notes"
            type="button"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.5" />
              <path d="M11 11L14.5 14.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
          <button
            className={styles.iconButton}
            onClick={handleCreateNote}
            aria-label="New note"
            type="button"
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
              <path d="M8 2V14M2 8H14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      </div>
      <div className={styles.list}>
        {isLoading && notes.length === 0 ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              Loading...
            </Text>
          </div>
        ) : notes.length === 0 ? (
          <div className={styles.empty}>
            <Text as="div" size="sm" tone="secondary">
              No notes yet
            </Text>
            <Text as="div" size="sm" tone="secondary">
              Create your first note to get started
            </Text>
          </div>
        ) : (
          notes.map((note) => (
            <button
              key={note.id}
              className={`${styles.noteItem} ${
                note.id === selectedNoteId ? styles.noteItemSelected : ""
              }`}
              onClick={() => handleSelectNote(note)}
              type="button"
            >
              <div className={styles.noteLine1}>
                <span className={styles.noteTitle}>
                  {note.title || "Untitled"}
                </span>
              </div>
              <div className={styles.noteLine2}>
                <span className={styles.noteDate}>
                  {formatRelativeTime(note.modifiedAt)}
                </span>
                {note.preview && (
                  <span className={styles.notePreview}>{note.preview}</span>
                )}
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
