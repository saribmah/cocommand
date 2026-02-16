import { NotesSidebar } from "./components/NotesSidebar";
import { NoteEditor } from "./components/NoteEditor";
import styles from "./notes.module.css";
import type { ExtensionViewProps } from "../extension/extension-views";

export function NotesView({ mode }: ExtensionViewProps) {
  const isInline = mode === "inline";
  return (
    <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
      <NotesSidebar />
      <NoteEditor />
    </div>
  );
}
