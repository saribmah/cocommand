import type { ExtensionPartInput, FilePartInput } from "./command.types";

export interface ComposerActions {
  /** Add a file or extension part to the composer draft */
  addPart: (part: FilePartInput | ExtensionPartInput) => void;
  /** Remove a tagged part by type and name */
  removePart: (match: { type: "extension" | "file"; name: string }) => void;
  /** Switch the active filter tab */
  setActiveTab: (tab: string) => void;
  /** Focus the main input field */
  focusInput: () => void;
}
