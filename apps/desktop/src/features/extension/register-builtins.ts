import { registerExtensionView } from "./extension-views";
import { registerExtensionStore } from "./extension-stores";
import { NotesView } from "../notes/notes.view";
import { FileSystemView } from "../filesystem/filesystem.view";
import { ClipboardView } from "../clipboard/clipboard.view";
import { ScreenshotView } from "../screenshot/screenshot.view";
import { WorkspaceView } from "../workspace/workspace.view";
import { createFileSystemStore } from "../filesystem/filesystem.store";
import { createNotesStore } from "../notes/notes.store";
import { createClipboardStore } from "../clipboard/clipboard.store";
import { createScreenshotStore } from "../screenshot/screenshot.store";
import { createWorkspaceExtensionStore } from "../workspace/workspace.extension-store";

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

registerExtensionView("clipboard", {
  component: ClipboardView,
  label: "Clipboard",
  popout: { width: 750, height: 500, title: "Clipboard History" },
});
registerExtensionStore("clipboard", createClipboardStore);

registerExtensionView("screenshot", {
  component: ScreenshotView,
  label: "Screenshots",
  popout: { width: 900, height: 600, title: "Screenshots" },
});
registerExtensionStore("screenshot", createScreenshotStore);

registerExtensionView("workspace", {
  component: WorkspaceView,
  label: "Settings",
  popout: { width: 720, height: 520, title: "Settings" },
});
registerExtensionStore("workspace", createWorkspaceExtensionStore);
