import { NotesSidebar } from "./components/NotesSidebar";
import { NoteEditor } from "./components/NoteEditor";
import styles from "./notes.module.css";

export function NotesView() {
  return (
    <div className={`app-shell ${styles.container}`}>
      <NotesSidebar />
      <NoteEditor />
    </div>
  );
}
