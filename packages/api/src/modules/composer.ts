import type { ComposerActionsBridge } from "../configure";
import type { FilePartInput, ExtensionPartInput } from "../types";

export interface ComposerApi {
  addFile(part: FilePartInput): void;
  addExtension(part: ExtensionPartInput): void;
  removePart(match: { type: "extension" | "file"; name: string }): void;
  setActiveTab(tab: string): void;
  focusInput(): void;
}

export function createComposer(bridge?: ComposerActionsBridge): ComposerApi {
  function getBridge(): ComposerActionsBridge {
    if (!bridge) {
      throw new Error(
        "@cocommand/api: Composer is only available in view context with a composer bridge.",
      );
    }
    return bridge;
  }

  return {
    addFile(part) {
      getBridge().addPart(part);
    },
    addExtension(part) {
      getBridge().addPart(part);
    },
    removePart(match) {
      getBridge().removePart(match);
    },
    setActiveTab(tab) {
      getBridge().setActiveTab(tab);
    },
    focusInput() {
      getBridge().focusInput();
    },
  };
}
