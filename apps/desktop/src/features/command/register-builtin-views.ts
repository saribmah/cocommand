import { registerExtensionView } from "./extension-views";
import { registerExtensionStore } from "../extension/extension-stores";
import { NotesView } from "../notes/notes.view";
import { FileSystemView } from "../filesystem/filesystem.view";
import { createFileSystemStore } from "../filesystem/filesystem.store";
import { createNotesStore } from "../notes/notes.store";

registerExtensionView("notes", {
  component: NotesView,
  label: "Notes",
  popout: { width: 800, height: 600, title: "Notes" },
});
registerExtensionStore("notes", createNotesStore);

registerExtensionView("filesystem", {
  component: FileSystemView,
  label: "Files",
  popout: { width: 700, height: 500, title: "Files" },
});
registerExtensionStore("filesystem", createFileSystemStore);
