import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
} from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "@tiptap/markdown";
import Placeholder from "@tiptap/extension-placeholder";
import type { EditorProps, EditorRef } from "./editor.types";
import styles from "./editor.module.css";

/**
 * A reusable rich text editor component powered by Tiptap.
 *
 * Supports markdown input/output, making it suitable for:
 * - Editing notes
 * - Opening markdown files
 * - General rich text editing
 *
 * @example
 * ```tsx
 * const editorRef = useRef<EditorRef>(null);
 *
 * <Editor
 *   content="# Hello\n\nThis is **markdown**"
 *   onChange={(md) => console.log(md)}
 *   placeholder="Start writing..."
 * />
 * ```
 */
export const Editor = forwardRef<EditorRef, EditorProps>(function Editor(
  {
    content = "",
    onChange,
    editable = true,
    placeholder = "Start writing...",
    className,
    autoFocus = false,
  },
  ref
) {
  const initialContentRef = useRef(content);
  const onChangeRef = useRef(onChange);

  // Keep onChange ref updated
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        // Disable history if needed for collaborative editing later
        // history: false,
      }),
      Markdown.configure({
        // Use GitHub Flavored Markdown
        markedOptions: {
          gfm: true,
          breaks: false,
        },
      }),
      Placeholder.configure({
        placeholder,
        emptyEditorClass: "is-editor-empty",
      }),
    ],
    content,
    contentType: "markdown",
    editable,
    autofocus: autoFocus ? "end" : false,
    onUpdate: ({ editor }) => {
      if (onChangeRef.current) {
        const markdown = editor.getMarkdown();
        onChangeRef.current(markdown);
      }
    },
  });

  // Update editable state when prop changes
  useEffect(() => {
    if (editor && editor.isEditable !== editable) {
      editor.setEditable(editable);
    }
  }, [editor, editable]);

  // Get current markdown content
  const getMarkdown = useCallback((): string => {
    if (!editor) return "";
    return editor.getMarkdown();
  }, [editor]);

  // Set content from markdown
  const setMarkdown = useCallback(
    (markdown: string): void => {
      if (!editor) return;
      editor.commands.setContent(markdown, {
        emitUpdate: false,
        contentType: "markdown",
      });
      initialContentRef.current = markdown;
    },
    [editor]
  );

  // Focus the editor
  const focus = useCallback((): void => {
    editor?.commands.focus("end");
  }, [editor]);

  // Check if content has changed
  const isDirty = useCallback((): boolean => {
    if (!editor) return false;
    const current = getMarkdown();
    return current !== initialContentRef.current;
  }, [editor, getMarkdown]);

  // Expose ref methods
  useImperativeHandle(
    ref,
    () => ({
      getMarkdown,
      setMarkdown,
      getEditor: () => editor,
      focus,
      isDirty,
    }),
    [getMarkdown, setMarkdown, editor, focus, isDirty]
  );

  const containerClasses = [
    styles.editor,
    !editable && styles.readOnly,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={containerClasses}>
      <EditorContent editor={editor} className={styles.editorContent} />
    </div>
  );
});
