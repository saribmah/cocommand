import { useCallback, useEffect, useRef, useState } from "react";
import { Text } from "@cocommand/ui";
import { Editor, type EditorRef } from "../../editor";
import { useNotesContext } from "../notes.context";
import styles from "./NoteEditor.module.css";

/** Debounce delay for auto-save in milliseconds */
const AUTO_SAVE_DELAY = 1000;

export function NoteEditor() {
  const selectedNote = useNotesContext((state) => state.selectedNote);
  const selectedNoteId = useNotesContext((state) => state.selectedNoteId);
  const isLoading = useNotesContext((state) => state.isLoading);
  const isSaving = useNotesContext((state) => state.isSaving);
  const updateNote = useNotesContext((state) => state.updateNote);

  const editorRef = useRef<EditorRef>(null);
  const saveTimeoutRef = useRef<number | null>(null);
  const [lastSavedContent, setLastSavedContent] = useState<string>("");

  // Update lastSavedContent when a new note is loaded
  useEffect(() => {
    if (selectedNote) {
      setLastSavedContent(selectedNote.content);
    }
  }, [selectedNote?.id]); // Only reset when note ID changes

  const handleSave = useCallback(
    async (content: string) => {
      if (!selectedNoteId || content === lastSavedContent) {
        return;
      }

      try {
        await updateNote(selectedNoteId, content);
        setLastSavedContent(content);
      } catch (error) {
        console.error("Failed to save note:", error);
      }
    },
    [selectedNoteId, lastSavedContent, updateNote]
  );

  const handleChange = useCallback(
    (markdown: string) => {
      // Clear existing timeout
      if (saveTimeoutRef.current !== null) {
        window.clearTimeout(saveTimeoutRef.current);
      }

      // Set new timeout for auto-save
      saveTimeoutRef.current = window.setTimeout(() => {
        handleSave(markdown);
      }, AUTO_SAVE_DELAY);
    },
    [handleSave]
  );

  // Clean up timeout on unmount
  useEffect(() => {
    return () => {
      if (saveTimeoutRef.current !== null) {
        window.clearTimeout(saveTimeoutRef.current);
      }
    };
  }, []);

  // Save before switching notes
  useEffect(() => {
    return () => {
      if (saveTimeoutRef.current !== null) {
        window.clearTimeout(saveTimeoutRef.current);
        // Immediate save on unmount/note change
        const content = editorRef.current?.getMarkdown();
        if (content && content !== lastSavedContent && selectedNoteId) {
          // Fire and forget - we're unmounting
          updateNote(selectedNoteId, content).catch(console.error);
        }
      }
    };
  }, [selectedNoteId, lastSavedContent, updateNote]);

  if (!selectedNoteId) {
    return (
      <div className={styles.empty}>
        <Text as="div" size="md" tone="secondary">
          Select a note to view or edit
        </Text>
        <Text as="div" size="sm" tone="secondary">
          Or create a new note to get started
        </Text>
      </div>
    );
  }

  if (isLoading && !selectedNote) {
    return (
      <div className={styles.loading}>
        <Text as="div" size="md" tone="secondary">
          Loading...
        </Text>
      </div>
    );
  }

  if (!selectedNote) {
    return (
      <div className={styles.empty}>
        <Text as="div" size="md" tone="secondary">
          Note not found
        </Text>
      </div>
    );
  }

  return (
    <div className={styles.editor}>
      <div className={styles.header}>
        <Text as="div" size="lg" weight="semibold" className={styles.title}>
          {selectedNote.title}
        </Text>
        {isSaving && (
          <Text as="span" size="xs" tone="secondary">
            Saving...
          </Text>
        )}
      </div>
      <div className={styles.content}>
        <Editor
          ref={editorRef}
          key={selectedNote.id}
          content={selectedNote.content}
          onChange={handleChange}
          placeholder="Start writing..."
          autoFocus
        />
      </div>
    </div>
  );
}
