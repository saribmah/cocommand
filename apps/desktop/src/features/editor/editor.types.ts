import type { Editor } from "@tiptap/react";

/**
 * Props for the core Editor component
 */
export interface EditorProps {
  /** Initial content as markdown string */
  content?: string;
  /** Called when content changes, provides markdown string */
  onChange?: (markdown: string) => void;
  /** Whether the editor is editable (default: true) */
  editable?: boolean;
  /** Placeholder text shown when editor is empty */
  placeholder?: string;
  /** Additional CSS class name */
  className?: string;
  /** Auto-focus the editor on mount */
  autoFocus?: boolean;
}

/**
 * Ref handle for the Editor component
 */
export interface EditorRef {
  /** Get current content as markdown */
  getMarkdown: () => string;
  /** Set content from markdown */
  setMarkdown: (markdown: string) => void;
  /** Get the underlying Tiptap editor instance */
  getEditor: () => Editor | null;
  /** Focus the editor */
  focus: () => void;
  /** Check if content has changed from initial value */
  isDirty: () => boolean;
}
