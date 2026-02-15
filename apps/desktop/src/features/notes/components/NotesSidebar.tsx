import { useEffect } from "react";
import { ListItem, ButtonPrimary, Text } from "@cocommand/ui";
import { useNotesContext } from "../notes.context";
import type { NoteSummary } from "../notes.types";
import styles from "./NotesSidebar.module.css";

function formatRelativeTime(timestamp: number | null): string {
  if (!timestamp) return "";
  const now = Date.now();
  const diff = now - timestamp * 1000;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return "Just now";
}

export function NotesSidebar() {
  const notes = useNotesContext((state) => state.notes);
  const selectedNoteId = useNotesContext((state) => state.selectedNoteId);
  const isLoading = useNotesContext((state) => state.isLoading);
  const fetchNotes = useNotesContext((state) => state.fetchNotes);
  const selectNote = useNotesContext((state) => state.selectNote);
  const createNote = useNotesContext((state) => state.createNote);

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
          Notes
        </Text>
        <ButtonPrimary onClick={handleCreateNote}>New</ButtonPrimary>
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
            <ListItem
              key={note.id}
              title={note.title}
              subtitle={note.preview || undefined}
              rightMeta={
                <Text as="span" size="xs" tone="secondary">
                  {formatRelativeTime(note.modifiedAt)}
                </Text>
              }
              selected={note.id === selectedNoteId}
              onClick={() => handleSelectNote(note)}
              className={styles.noteItem}
            />
          ))
        )}
      </div>
    </div>
  );
}
