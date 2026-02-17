import type { FilePartInput, ExtensionPartInput } from "./types";

export interface ComposerActionsBridge {
  addPart: (part: FilePartInput | ExtensionPartInput) => void;
  removePart: (match: { type: "extension" | "file"; name: string }) => void;
  setActiveTab: (tab: string) => void;
  focusInput: () => void;
}
