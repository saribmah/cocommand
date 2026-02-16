import { registerExtensionView } from "./extension-views";
import { NotesView } from "../notes/notes.view";
import { FileSystemView } from "../filesystem/filesystem.view";

registerExtensionView("notes", {
  component: NotesView,
  label: "Notes",
  popout: { width: 800, height: 600, title: "Notes" },
});

registerExtensionView("filesystem", {
  component: FileSystemView,
  label: "Files",
  popout: { width: 700, height: 500, title: "Files" },
});
