import { NotesSidebar } from "./components/NotesSidebar";
import { NoteEditor } from "./components/NoteEditor";
import styles from "./notes.module.css";
import type { ExtensionViewProps } from "../command/extension-views";

interface NotesViewProps extends Partial<ExtensionViewProps> {}

export function NotesView({ mode }: NotesViewProps) {
  const isInline = mode === "inline";
  return (
    <div className={`${isInline ? "" : "app-shell "}${styles.container}${isInline ? ` ${styles.inline}` : ""}`}>
      <NotesSidebar />
      <NoteEditor />
    </div>
  );
}
