import { useCallback, useEffect, useRef, useState } from "react";
import { Text } from "@cocommand/ui";
import { Editor, type EditorRef } from "../../editor";
import { useNotesContext } from "../notes.context";
import type { Editor as TiptapEditor } from "@tiptap/react";
import styles from "./NoteEditor.module.css";

/** Debounce delay for auto-save in milliseconds */
const AUTO_SAVE_DELAY = 1000;

function formatNoteDate(timestamp: number | null): string {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString("en-US", {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
  }) + " at " + date.toLocaleTimeString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}

interface ToolbarButtonProps {
  label: string;
  onClick: () => void;
  isActive?: boolean;
  title?: string;
}

function ToolbarButton({ label, onClick, isActive, title }: ToolbarButtonProps) {
  return (
    <button
      className={`${styles.toolbarButton} ${isActive ? styles.toolbarButtonActive : ""}`}
      onClick={onClick}
      title={title || label}
      type="button"
    >
      {label}
    </button>
  );
}

function FormatToolbar({ editor }: { editor: TiptapEditor | null }) {
  if (!editor) return null;

  return (
    <div className={styles.toolbar}>
      <div className={styles.toolbarGroup}>
        <ToolbarButton
          label="B"
          onClick={() => editor.chain().focus().toggleBold().run()}
          isActive={editor.isActive("bold")}
        />
        <ToolbarButton
          label="I"
          onClick={() => editor.chain().focus().toggleItalic().run()}
          isActive={editor.isActive("italic")}
        />
        <ToolbarButton
          label="S"
          onClick={() => editor.chain().focus().toggleStrike().run()}
          isActive={editor.isActive("strike")}
        />
      </div>
      <div className={styles.toolbarDivider} />
      <div className={styles.toolbarGroup}>
        <ToolbarButton
          label="H1"
          onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
          isActive={editor.isActive("heading", { level: 1 })}
        />
        <ToolbarButton
          label="H2"
          onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
          isActive={editor.isActive("heading", { level: 2 })}
        />
        <ToolbarButton
          label="H3"
          onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
          isActive={editor.isActive("heading", { level: 3 })}
        />
        <ToolbarButton
          label="H4"
          onClick={() => editor.chain().focus().toggleHeading({ level: 4 }).run()}
          isActive={editor.isActive("heading", { level: 4 })}
        />
      </div>
      <div className={styles.toolbarDivider} />
      <div className={styles.toolbarGroup}>
        <ToolbarButton
          label="•"
          title="Bullet List"
          onClick={() => editor.chain().focus().toggleBulletList().run()}
          isActive={editor.isActive("bulletList")}
        />
        <ToolbarButton
          label="1."
          title="Ordered List"
          onClick={() => editor.chain().focus().toggleOrderedList().run()}
          isActive={editor.isActive("orderedList")}
        />
      </div>
      <div className={styles.toolbarDivider} />
      <div className={styles.toolbarGroup}>
        <ToolbarButton
          label="❝"
          title="Block Quote"
          onClick={() => editor.chain().focus().toggleBlockquote().run()}
          isActive={editor.isActive("blockquote")}
        />
        <ToolbarButton
          label="{ }"
          title="Code Block"
          onClick={() => editor.chain().focus().toggleCodeBlock().run()}
          isActive={editor.isActive("codeBlock")}
        />
        <ToolbarButton
          label="<>"
          title="Inline Code"
          onClick={() => editor.chain().focus().toggleCode().run()}
          isActive={editor.isActive("code")}
        />
      </div>
      <div className={styles.toolbarDivider} />
      <div className={styles.toolbarGroup}>
        <ToolbarButton
          label="—"
          title="Horizontal Rule"
          onClick={() => editor.chain().focus().setHorizontalRule().run()}
        />
      </div>
    </div>
  );
}

export function NoteEditor() {
  const selectedNote = useNotesContext((state) => state.selectedNote);
  const selectedNoteId = useNotesContext((state) => state.selectedNoteId);
  const isLoading = useNotesContext((state) => state.isLoading);
  const isSaving = useNotesContext((state) => state.isSaving);
  const updateNote = useNotesContext((state) => state.updateNote);

  const editorRef = useRef<EditorRef>(null);
  const saveTimeoutRef = useRef<number | null>(null);
  const [lastSavedContent, setLastSavedContent] = useState<string>("");
  const [title, setTitle] = useState<string>("");
  const [tiptapEditor, setTiptapEditor] = useState<TiptapEditor | null>(null);

  // Update title and lastSavedContent when a new note is loaded
  useEffect(() => {
    if (selectedNote) {
      setLastSavedContent(selectedNote.content);
      setTitle(selectedNote.title);
    }
  }, [selectedNote?.id]); // Only reset when note ID changes

  // Grab the tiptap editor instance once it's ready
  useEffect(() => {
    const interval = setInterval(() => {
      const ed = editorRef.current?.getEditor();
      if (ed) {
        setTiptapEditor(ed);
        clearInterval(interval);
      }
    }, 50);
    return () => clearInterval(interval);
  }, [selectedNote?.id]);

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

  const scheduleAutoSave = useCallback(
    (content: string) => {
      if (saveTimeoutRef.current !== null) {
        window.clearTimeout(saveTimeoutRef.current);
      }
      saveTimeoutRef.current = window.setTimeout(() => {
        handleSave(content);
      }, AUTO_SAVE_DELAY);
    },
    [handleSave]
  );

  const handleChange = useCallback(
    (markdown: string) => {
      scheduleAutoSave(markdown);
    },
    [scheduleAutoSave]
  );

  const handleTitleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newTitle = e.target.value;
      setTitle(newTitle);

      // Rebuild content: replace the first H1 line or prepend one
      const currentContent = editorRef.current?.getMarkdown() || "";
      let updatedContent: string;

      // Check if content starts with an H1
      if (/^# .+/.test(currentContent)) {
        updatedContent = currentContent.replace(/^# .+/, `# ${newTitle}`);
      } else {
        updatedContent = `# ${newTitle}\n\n${currentContent}`;
      }

      // Set the editor content and schedule save
      editorRef.current?.setMarkdown(updatedContent);
      scheduleAutoSave(updatedContent);
    },
    [scheduleAutoSave]
  );

  const handleTitleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        e.preventDefault();
        editorRef.current?.focus();
      }
    },
    []
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
        const content = editorRef.current?.getMarkdown();
        if (content && content !== lastSavedContent && selectedNoteId) {
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
      <div className={styles.dateBar}>
        <span className={styles.dateText}>
          {formatNoteDate(selectedNote.modifiedAt)}
        </span>
        {isSaving && (
          <span className={styles.savingIndicator}>Saving...</span>
        )}
      </div>
      <FormatToolbar editor={tiptapEditor} />
      <div className={styles.titleArea}>
        <input
          className={styles.titleInput}
          type="text"
          value={title}
          onChange={handleTitleChange}
          onKeyDown={handleTitleKeyDown}
          placeholder="Untitled"
        />
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
